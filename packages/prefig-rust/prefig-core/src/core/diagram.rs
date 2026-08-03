//! Port of prefig/core/diagram.py: the Diagram class that owns the SVG tree,
//! coordinate-transform stack, ids, defs/reusables, labels, and annotations.

use crate::core::ctm::{AxisScale, CTM};
use crate::core::label::LabelState;
use crate::core::utilities::float2str;
use crate::core::{label, repeat, tags};
use crate::evaluator::ExpressionContext;
use crate::value::{py_str, Value};
use crate::xml::{self, El};
use std::collections::HashMap;

pub type Point = [f64; 2];

fn el_key(el: &El) -> usize {
    std::rc::Rc::as_ptr(el) as usize
}

fn epub_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub struct Diagram {
    pub ctx: ExpressionContext,
    pub labels: LabelState,

    pub diagram_element: El,
    pub filename: String,
    // Retained to mirror the Python Diagram; not yet read by any Rust code path.
    #[allow(dead_code)]
    diagram_number: Option<i64>,
    format: String,
    environment: String,
    suppress_caption: bool,
    caption: String,

    pub root: El,
    defs: El,

    add_id_prefix: bool,
    id_prefix: String,
    id_suffix: Vec<String>,
    ids: HashMap<String, i64>,

    ctm_stack: Vec<(CTM, [f64; 4])>,
    scale_stack: Vec<[AxisScale; 2]>,
    pub margins: [f64; 4],
    clippaths: Vec<String>,
    reusables: indexmap::IndexMap<String, El>,
    textures: indexmap::IndexMap<String, String>,

    // (label source element, group, CTM at registration)
    pub label_group_dict: Vec<(El, El, CTM)>,
    label_dims: HashMap<usize, (f64, f64)>,
    // In native label mode, per-label records of the host-rendered runs a label
    // pushed to `labels.placements`, so a <legend> can re-anchor them after it
    // lays its items out (the placement was recorded at the label's own anchor,
    // before the legend layout existed). Keyed by el_key(label); each entry is
    // (index into labels.placements, label-group-local baseline point).
    native_label_runs: HashMap<usize, Vec<(usize, [f64; 2])>>,
    // In native label mode, a transform some caller wrapped a label's `<g>` in
    // (e.g. a <line> label placed inside `translate(q1) rotate(-angle)`). SVG
    // applies it at render time, but native runs are lifted out with absolute
    // coordinates, so `position_svg_label` composes this into each run's
    // placement. Keyed by el_key(label). See `line::add_label`.
    native_label_wrappers: HashMap<usize, crate::core::ctm::Mat2x3>,
    saved_data: HashMap<usize, ((Point, Point), ())>,

    pub defaults: indexmap::IndexMap<String, El>,
    external: Option<String>,

    // annotations
    annotations_root: Option<El>,
    default_annotations: Vec<El>,
    annotation_branches: indexmap::IndexMap<String, El>,
    annotation_branch_stack: Vec<El>,
    pub author_annotations_present: bool,
    add_default_annotations: bool,

    // tactile page geometry
    pub centerline: f64,
    pub bottomline: f64,

    // the <axes> most recently built (Python: axes.axes_object module global)
    pub axes_info: Option<crate::core::axes::AxesInfo>,

    // arrow.arrow_length_dict: marker id -> tip length
    pub arrow_lengths: HashMap<String, f64>,

    // register_source_data/get_source_data: per-element computed values
    source_data: HashMap<usize, indexmap::IndexMap<String, Value>>,

    // pyodide default annotations (diagram_to_speech): a pristine deep copy of
    // the source diagram made before parsing, a map from each source element to
    // its copy, and a map from each copy to the SVG element it produced.
    diagram_element_copy: El,
    source_to_copy: HashMap<usize, El>,
    source_to_svg: HashMap<usize, El>,

    // named shapes from <define-shapes>
    shape_dict: indexmap::IndexMap<String, El>,

    // legends to place after labels are rendered
    pub legends: Vec<crate::core::legend::LegendData>,
}

impl Diagram {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        diagram_element: El,
        filename: &str,
        diagram_number: Option<i64>,
        format: &str,
        publication: Option<El>,
        suppress_caption: bool,
        environment: &str,
        labels: LabelState,
    ) -> Diagram {
        let root = xml::new_element("svg");
        root.borrow_mut()
            .set("xmlns", "http://www.w3.org/2000/svg");

        let mut add_id_prefix = false;
        let mut id_prefix = String::new();
        let mut figure_id = "figure".to_string();
        if format != "tactile" {
            add_id_prefix = true;
            if environment == "pyodide" {
                // Python uses time_ns + hash; we only need uniqueness among the
                // diagrams built on one page, and SystemTime panics on
                // wasm32-unknown-unknown, so use a process-wide counter.
                use std::sync::atomic::{AtomicU64, Ordering};
                static COUNTER: AtomicU64 = AtomicU64::new(0);
                let n = COUNTER.fetch_add(1, Ordering::Relaxed);
                id_prefix = format!("prefig-{n:x}-");
            } else {
                let stem = std::path::Path::new(filename)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                id_prefix = repeat::epub_clean(&format!("{stem}-"));
            }
            figure_id = format!("{id_prefix}{figure_id}");
        }

        // A pristine copy of the source, made before parsing mutates the tree,
        // plus a source-element -> copy-element map (parallel document-order
        // iteration). Used only for pyodide default annotations.
        let diagram_element_copy = xml::deep_copy(&diagram_element);
        let mut source_to_copy: HashMap<usize, El> = HashMap::new();
        for (source, copy) in xml::iter_subtree(&diagram_element)
            .iter()
            .zip(xml::iter_subtree(&diagram_element_copy).iter())
        {
            source_to_copy.insert(el_key(source), copy.clone());
        }

        // Warn when a boolean attribute uses "true"/"false" instead of
        // PreFigure's "yes"/"no" convention. Elements inside <annotations> are
        // exempt (their values are free-form speech/text). Mirrors
        // prefig/core/diagram.py.
        let mut excluded: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for ann in xml::find_descendants(&diagram_element, "annotations") {
            for el in xml::iter_subtree(&ann) {
                excluded.insert(el_key(&el));
            }
        }
        for el in xml::iter_subtree(&diagram_element) {
            if excluded.contains(&el_key(&el)) {
                continue;
            }
            let el = el.borrow();
            for (attr, val) in &el.attrs {
                if val == "true" || val == "false" {
                    log::warn!(
                        "<{}> attribute {attr}=\"{val}\": PreFigure uses \"yes\"/\"no\" for boolean attributes",
                        el.tag
                    );
                }
            }
        }

        let mut diagram = Diagram {
            ctx: ExpressionContext::new(),
            labels,
            diagram_element: diagram_element.clone(),
            filename: filename.to_string(),
            diagram_number,
            format: format.to_string(),
            environment: environment.to_string(),
            suppress_caption,
            caption: String::new(),
            root: root.clone(),
            defs: xml::new_element("defs"), // replaced in begin_figure
            add_id_prefix,
            id_prefix,
            id_suffix: vec![String::new()],
            ids: HashMap::new(),
            ctm_stack: Vec::new(),
            scale_stack: Vec::new(),
            margins: [0.0; 4],
            clippaths: Vec::new(),
            reusables: indexmap::IndexMap::new(),
            textures: indexmap::IndexMap::new(),
            label_group_dict: Vec::new(),
            label_dims: HashMap::new(),
            native_label_runs: HashMap::new(),
            native_label_wrappers: HashMap::new(),
            saved_data: HashMap::new(),
            defaults: indexmap::IndexMap::new(),
            external: None,
            annotations_root: None,
            default_annotations: Vec::new(),
            annotation_branches: indexmap::IndexMap::new(),
            annotation_branch_stack: Vec::new(),
            author_annotations_present: false,
            add_default_annotations: environment != "pyodide"
                || !xml::find_descendants(&diagram_element, "annotations").is_empty(),
            centerline: 0.0,
            bottomline: 0.0,
            axes_info: None,
            arrow_lengths: HashMap::new(),
            source_data: HashMap::new(),
            diagram_element_copy,
            source_to_copy,
            source_to_svg: HashMap::new(),
            shape_dict: indexmap::IndexMap::new(),
            legends: Vec::new(),
        };

        {
            let id = diagram_element
                .borrow()
                .get("id")
                .unwrap_or(figure_id);
            diagram.add_id(&root, Some(&id));
        }

        // read defaults from the publication file
        if let Some(publication) = publication {
            let children: Vec<El> = publication.borrow().children.clone();
            for sub in children {
                let tag = sub.borrow().tag.clone();
                if tag == "external-root" {
                    log::warn!("<external-root> in publication file is deprecated");
                    if let Some(name) = sub.borrow().get("name") {
                        diagram.external = Some(name);
                    }
                    continue;
                }
                diagram.defaults.insert(tag, sub);
            }
            if let Some(directories) = xml::find_descendants(&publication, "directories").first() {
                if let Some(data_dir) = directories.borrow().get("data") {
                    diagram.external = Some(data_dir);
                }
            }
        }

        // <templates> inside the diagram add defaults, then are removed
        let templates = xml::find_descendants(&diagram_element, "templates");
        if let Some(templates_element) = templates.first() {
            for template in &templates {
                if let Some(parent) = xml::get_parent(template) {
                    xml::remove(&parent, template);
                }
            }
            let children: Vec<El> = templates_element.borrow().children.clone();
            for child in children {
                let tag = child.borrow().tag.clone();
                diagram.defaults.insert(tag, child);
            }
        }

        let author_annotations = xml::find_descendants(&diagram_element, "annotations");
        diagram.author_annotations_present = !author_annotations.is_empty();
        if let Some(annotations) = author_annotations.first() {
            diagram.check_annotation_ref(annotations);
        }

        if let Some(macros) = diagram.defaults.get("macros").cloned() {
            let text = macros.borrow().text.clone().unwrap_or_default();
            diagram.labels.math.add_macros(&text);
        }

        diagram
    }

    fn check_annotation_ref(&self, element: &El) {
        let ref_attr = element.borrow().get("ref");
        if let Some(ref_attr) = ref_attr {
            if !epub_id_ok(&ref_attr) {
                log::error!("@ref {ref_attr} in an annotation has characters disallowed by EPUB");
                element
                    .borrow_mut()
                    .set("ref", &repeat::epub_clean(&ref_attr));
            }
        }
        let children: Vec<El> = element.borrow().children.clone();
        for child in &children {
            self.check_annotation_ref(child);
        }
    }

    // ---------- labels ----------

    pub fn add_label(&mut self, element: &El, group: &El) {
        let ctm = self.ctm().clone();
        self.label_group_dict
            .push((element.clone(), group.clone(), ctm));
    }

    pub fn register_label_dims(&mut self, element: &El, dims: (f64, f64)) {
        self.label_dims.insert(el_key(element), dims);
    }

    pub fn get_label_dims(&self, element: &El) -> Option<(f64, f64)> {
        self.label_dims.get(&el_key(element)).copied()
    }

    /// Native mode: remember that `element` pushed a host-rendered run at
    /// `placement_index` in `labels.placements`, whose baseline sits at `local`
    /// in the label group's own coordinates. A `<legend>` uses this to move the
    /// run once it knows where the item finally lands. See `legend::place_legend`.
    pub fn record_native_run(&mut self, element: &El, placement_index: usize, local: [f64; 2]) {
        self.native_label_runs
            .entry(el_key(element))
            .or_default()
            .push((placement_index, local));
    }

    /// The native runs `element` recorded via [`record_native_run`], if any.
    pub fn native_runs_for(&self, element: &El) -> Vec<(usize, [f64; 2])> {
        self.native_label_runs
            .get(&el_key(element))
            .cloned()
            .unwrap_or_default()
    }

    /// Register a transform that wraps `element`'s label `<g>` in the SVG, so
    /// native placements can compose it (see `native_wrapper_for`).
    pub fn set_native_wrapper(&mut self, element: &El, m: crate::core::ctm::Mat2x3) {
        self.native_label_wrappers.insert(el_key(element), m);
    }

    /// The wrapper transform registered for `element` via [`set_native_wrapper`].
    pub fn native_wrapper_for(&self, element: &El) -> Option<crate::core::ctm::Mat2x3> {
        self.native_label_wrappers.get(&el_key(element)).copied()
    }

    /// diagram.get_label_group(element)[0]: the <g> registered for a label.
    pub fn get_label_group(&self, element: &El) -> Option<El> {
        self.label_group_dict
            .iter()
            .find(|(source, _, _)| std::rc::Rc::ptr_eq(source, element))
            .map(|(_, group, _)| group.clone())
    }

    pub fn set_caption(&mut self, text: &str) {
        self.caption = text.to_string();
    }

    pub fn caption_suppressed(&self) -> bool {
        self.suppress_caption
    }

    pub fn get_external(&self) -> Option<String> {
        self.external.clone()
    }

    // ---------- ids ----------

    pub fn add_id(&mut self, element: &El, id: Option<&str>) {
        let result = self.find_id(element, id);
        element.borrow_mut().set("id", &result);
    }

    pub fn find_id(&mut self, element: &El, id: Option<&str>) -> String {
        let suffix = self.id_suffix.concat();
        let result_id = match id {
            None => {
                let tag = element.borrow().tag.clone();
                let count = self.ids.entry(tag.clone()).and_modify(|c| *c += 1).or_insert(0);
                format!("__{tag}-{count}{suffix}")
            }
            Some(id) => {
                if !suffix.is_empty() && id.ends_with(&suffix) {
                    id.to_string()
                } else {
                    format!("{id}{suffix}")
                }
            }
        };
        self.prepend_id_prefix(&result_id)
    }

    pub fn append_id_suffix(&mut self, element: &El) -> String {
        let id = element.borrow().get("id");
        self.find_id(element, id.as_deref())
    }

    pub fn prepend_id_prefix(&self, id: &str) -> String {
        if !self.add_id_prefix || id.starts_with(&self.id_prefix) {
            return id.to_string();
        }
        format!("{}{}", self.id_prefix, id)
    }

    pub fn push_id_suffix(&mut self, suffix: &str) {
        self.id_suffix.push(suffix.to_string());
    }

    pub fn pop_id_suffix(&mut self) {
        self.id_suffix.pop();
    }

    pub fn output_format(&self) -> &str {
        &self.format
    }

    pub fn set_output_format(&mut self, format: &str) {
        self.format = format.to_string();
    }

    pub fn add_shape(&mut self, shape: &El) {
        xml::append(&self.defs, shape);
        let id = shape
            .borrow()
            .get("id")
            .or_else(|| shape.borrow().get("at"))
            .unwrap_or_default();
        self.shape_dict.insert(id, shape.clone());
    }

    pub fn recall_shape(&self, shape_id: &str) -> Option<El> {
        self.shape_dict.get(shape_id).cloned()
    }

    pub fn get_environment(&self) -> &str {
        &self.environment
    }

    // ---------- coordinate transforms ----------

    pub fn transform(&self, p: Point) -> Point {
        self.ctm_stack
            .last()
            .map(|(ctm, _)| ctm.transform(p))
            .unwrap_or([0.0, 0.0])
    }

    pub fn inverse_transform(&self, p: Point) -> Point {
        self.ctm_stack
            .last()
            .map(|(ctm, _)| ctm.inverse_transform(p))
            .unwrap_or([0.0, 0.0])
    }

    pub fn ctm(&mut self) -> &mut CTM {
        &mut self.ctm_stack.last_mut().expect("ctm stack is non-empty").0
    }

    pub fn ctm_ref(&self) -> &CTM {
        &self.ctm_stack.last().expect("ctm stack is non-empty").0
    }

    pub fn bbox(&self) -> [f64; 4] {
        self.ctm_stack.last().expect("ctm stack is non-empty").1
    }

    pub fn push_ctm(&mut self, ctm: CTM, bbox: [f64; 4]) {
        self.ctm_stack.push((ctm, bbox));
        self.sync_eval_env();
    }

    pub fn pop_ctm(&mut self) {
        self.ctm_stack.pop();
        self.sync_eval_env();
    }

    /// Give the expression evaluator access to the current bbox and 3-D
    /// projection (used by intersect(), proj_2d(), ...).
    fn sync_eval_env(&mut self) {
        if let Some((ctm, bbox)) = self.ctm_stack.last() {
            self.ctx.env_bbox = Some(*bbox);
            self.ctx.env_ctm3d = Some((ctm.ctm_3d, ctm.eye));
        }
    }

    pub fn push_scales(&mut self, scales: [AxisScale; 2]) {
        self.scale_stack.push(scales);
    }

    pub fn pop_scales(&mut self) {
        self.scale_stack.pop();
    }

    pub fn get_scales(&self) -> [AxisScale; 2] {
        *self
            .scale_stack
            .last()
            .unwrap_or(&[AxisScale::Linear, AxisScale::Linear])
    }

    pub fn get_margins(&self) -> [f64; 4] {
        self.margins
    }

    // ---------- clip paths ----------

    pub fn push_clippath(&mut self, clippath: El) {
        xml::append(&self.defs, &clippath);
        self.add_id(&clippath, None);
        let id = clippath.borrow().get("id").expect("id was just set");
        self.clippaths.push(id);
    }

    pub fn pop_clippath(&mut self) {
        self.clippaths.pop();
    }

    pub fn get_clippath(&self) -> String {
        self.clippaths.last().cloned().unwrap_or_default()
    }

    // ---------- saved data ----------

    pub fn save_line_endpoints(&mut self, element: &El, q1: Point, q2: Point) {
        self.saved_data.insert(el_key(element), ((q1, q2), ()));
    }

    pub fn retrieve_line_endpoints(&self, element: &El) -> Option<(Point, Point)> {
        self.saved_data.get(&el_key(element)).map(|(pts, _)| *pts)
    }

    /// register_svg_element: remember which SVG element a source element
    /// produced, so pyodide default annotations can link speech to graphics.
    /// The map is keyed by the source element's pristine copy (Python defaults
    /// overwrite=True).
    pub fn register_svg_element(&mut self, source: &El, svg: &El) {
        if let Some(copy) = self.source_to_copy.get(&el_key(source)) {
            self.source_to_svg.insert(el_key(copy), svg.clone());
        }
    }

    pub fn register_source_data(&mut self, element: &El, key: &str, value: Value) {
        self.source_data
            .entry(el_key(element))
            .or_default()
            .insert(key.to_string(), value);
    }

    pub fn get_source_data(&self, element: &El, key: &str) -> Option<Value> {
        self.source_data
            .get(&el_key(element))
            .and_then(|m| m.get(key).cloned())
    }

    // ---------- reusables / defs ----------

    pub fn get_defs(&self) -> El {
        self.defs.clone()
    }

    pub fn add_reusable(&mut self, element: &El) {
        let id = element.borrow().get_or("id", "none");
        if self.reusables.contains_key(&id) {
            return;
        }
        xml::append(&self.defs, element);
        self.reusables.insert(id, element.clone());
    }

    pub fn has_reusable(&self, id: &str) -> bool {
        self.reusables.contains_key(id)
    }

    pub fn get_reusable(&self, id: &str) -> Option<El> {
        self.reusables.get(id).cloned()
    }

    pub fn apply_defaults(&self, tag: &str, element: &El) {
        if let Some(default) = self.defaults.get(tag) {
            let attrs: Vec<(String, String)> = default
                .borrow()
                .attrs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            for (attr, value) in attrs {
                if element.borrow().get(&attr).is_none() {
                    element.borrow_mut().set(&attr, &value);
                }
            }
        }
    }

    // ---------- figure lifecycle ----------

    pub fn begin_figure(&mut self) -> Result<(), String> {
        let (width, height) = {
            let dims = self.diagram_element.borrow().get("dimensions");
            match dims {
                Some(dims) => {
                    let v = self
                        .ctx
                        .valid_eval(&dims)
                        .map_err(|e| format!("Unable to parse the dimensions of this diagram: {e}"))?;
                    let v = v.as_vec_f64().map_err(|e| e.to_string())?;
                    (v[0], v[1])
                }
                None => {
                    let w = self.diagram_element.borrow().get("width");
                    let h = self.diagram_element.borrow().get("height");
                    let w = self
                        .ctx
                        .valid_eval(&w.ok_or("diagram needs dimensions or width/height")?)
                        .map_err(|e| e.to_string())?
                        .as_num()
                        .map_err(|e| e.to_string())?;
                    let h = self
                        .ctx
                        .valid_eval(&h.ok_or("diagram needs dimensions or width/height")?)
                        .map_err(|e| e.to_string())?
                        .as_num()
                        .map_err(|e| e.to_string())?;
                    (w, h)
                }
            }
        };

        let mut margins_attr = self.diagram_element.borrow().get_or("margins", "[0,0,0,0]");
        if self.format == "tactile" {
            margins_attr = self
                .diagram_element
                .borrow()
                .get_or("tactile-margins", &margins_attr);
        }
        let margins_value = self
            .ctx
            .valid_eval(&margins_attr)
            .map_err(|e| format!("Unable to parse margins: {e}"))?;
        let margins: [f64; 4] = match &margins_value {
            Value::Array(_) => {
                let v = margins_value.as_vec_f64().map_err(|e| e.to_string())?;
                [v[0], v[1], v[2], v[3]]
            }
            other => {
                let m = other.as_num().map_err(|e| e.to_string())?;
                [m; 4]
            }
        };
        self.margins = margins;

        let mut ctm = CTM::new();
        if self.format == "tactile" {
            // tactile diagrams are embossed on 11.5" x 11" paper
            let total_width = width + margins[0] + margins[2];
            let total_height = height + margins[1] + margins[3];
            let diagram_aspect = total_width / total_height;
            let page_aspect = 10.5 / 8.8;

            let (s, lly) = if diagram_aspect >= page_aspect {
                let s = 756.0 / total_width;
                self.centerline = 378.0 + 36.0;
                (s, s * total_height + 79.2)
            } else {
                let s = 633.6 / total_height;
                self.centerline = s * total_width / 2.0 + 36.0;
                (s, 712.8)
            };
            self.bottomline = lly;
            ctm.translate(36.0, lly);
            ctm.scale(s, -s);
            ctm.translate(margins[0], margins[1]);
            self.root.borrow_mut().set("width", "828");
            self.root.borrow_mut().set("height", "792");
        } else {
            let w = width + margins[0] + margins[2];
            let h = height + margins[1] + margins[3];
            self.root.borrow_mut().set("width", &py_str(w));
            self.root.borrow_mut().set("height", &py_str(h));
            self.root
                .borrow_mut()
                .set("viewBox", &format!("0 0 {} {}", py_str(w), py_str(h)));

            ctm.translate(0.0, height + margins[1] + margins[3]);
            ctm.scale(1.0, -1.0);
            ctm.translate(margins[0], margins[1]);
        }

        let bbox = [0.0, 0.0, width, height];
        self.ctx.enter_namespace(
            "bbox",
            Value::Array(bbox.iter().map(|&b| Value::Num(b)).collect()),
        );
        self.ctm_stack = vec![(ctm, bbox)];
        self.scale_stack = vec![[AxisScale::Linear, AxisScale::Linear]];
        self.sync_eval_env();

        self.defs = xml::sub_element(&self.root, "defs");

        let clippath = xml::new_element("clipPath");
        let rect = xml::sub_element(&clippath, "rect");
        {
            let mut r = rect.borrow_mut();
            r.set("x", &float2str(margins[0]));
            r.set("y", &float2str(margins[3]));
            r.set("width", &float2str(width));
            r.set("height", &float2str(height));
        }
        self.push_clippath(clippath);
        Ok(())
    }

    /// Parse the children of `element`, placing SVG output under `root`.
    pub fn parse(&mut self, element: &El, root: &El, outline_group: Option<&El>) {
        let prefix = format!("{}-", self.format);
        let children: Vec<El> = element.borrow().children.clone();
        for child in children {
            // 'at' is the public name for 'id'
            let at = child.borrow().get("at");
            if let Some(at) = at {
                child.borrow_mut().set("id", &at);
            }
            let child_id = child.borrow().get("id");
            if let Some(child_id) = child_id {
                if !epub_id_ok(&child_id) {
                    log::error!("The id {child_id} has characters disallowed by EPUB");
                    child.borrow_mut().set("id", &repeat::epub_clean(&child_id));
                }
            }

            // publication-file defaults
            let tag = child.borrow().tag.clone();
            self.apply_defaults(&tag, &child);

            // format-specific attribute overrides (tactile-stroke, ...)
            let overrides: Vec<(String, String)> = child
                .borrow()
                .attrs
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(k, v)| (k[prefix.len()..].to_string(), v.clone()))
                .collect();
            for (attr, value) in overrides {
                child.borrow_mut().set(&attr, &value);
            }

            if let Err(e) = tags::parse_element(&child, self, root, outline_group) {
                log::error!("Error in parsing element {tag}: {e}");
                return;
            }

            // Re-read the tag: a handler may have retagged the element (e.g.
            // <riemann-sum> becomes <group> and builds its own annotation
            // branch). Python checks `child.tag` here, after parse_element, so
            // the retagged element is excluded and its rich branch is not
            // clobbered by a bare generated annotation.
            let current_tag = child.borrow().tag.clone();
            if child.borrow().get_or("annotate", "no") == "yes"
                && root.borrow().get_or("data-outline", "no") == "no"
                && current_tag != "group"
                && current_tag != "repeat"
            {
                let annotation = xml::new_element("annotation");
                for attrib in ["id", "text", "sonify", "circular", "speech"] {
                    if let Some(value) = child.borrow().get(attrib) {
                        annotation.borrow_mut().set(attrib, &value);
                    }
                }
                for attrib in ["text", "speech"] {
                    let value = annotation.borrow().get(attrib);
                    if let Some(value) = value {
                        let evaluated = label::evaluate_text(&value, &mut self.ctx);
                        annotation.borrow_mut().set(attrib, &evaluated);
                    }
                }
                self.add_annotation_to_branch(annotation);
            }
        }
    }

    pub fn place_labels(&mut self) {
        label::place_labels(self);

        let legends = std::mem::take(&mut self.legends);
        for legend in &legends {
            crate::core::legend::place_legend(self, legend);
        }

        if self.format == "tactile" {
            let caption = if self.caption.is_empty() {
                label::NEMETH_ON.to_string()
            } else {
                let translated = self
                    .labels
                    .braille
                    .translate(&self.caption, &vec![0; self.caption.len()])
                    .unwrap_or_default();
                format!("{} {}", translated, label::NEMETH_ON)
            };

            let gap = 3.6;
            let text_element = xml::sub_element(&self.root, "text");
            {
                let mut t = text_element.borrow_mut();
                t.text = Some(caption);
                t.set("x", "144");
                t.set("y", "50.4");
                t.set("font-family", "Braille29");
                t.set("font-size", "29px");
            }

            let text_element = xml::sub_element(&self.root, "text");
            {
                let mut t = text_element.borrow_mut();
                t.text = Some(label::NEMETH_OFF.to_string());
                t.set("x", "36");
                let y = label::snap_to_embossing_grid(self.bottomline + 12.0 * gap);
                t.set("y", &float2str(y));
                t.set("font-family", "Braille29");
                t.set("font-size", "29px");
            }
        }
    }

    pub fn annotate_source(&mut self) {
        // In the pyodide environment with no author annotations, generate
        // speech annotations directly from the (pristine) source tree.
        if self.environment != "pyodide" {
            return;
        }
        if !xml::find_descendants(&self.diagram_element, "annotations").is_empty() {
            return;
        }
        let copy = self.diagram_element_copy.clone();
        crate::core::annotations::diagram_to_speech(&copy, &self.source_to_svg);
        let root = xml::new_element("annotations");
        xml::append(&root, &copy);
        // Python passes parent=None here; annotations() only forwards parent in
        // the text-shortcut branch, which this generated tree never triggers.
        let dummy_parent = xml::new_element("none");
        crate::core::annotations::annotations(&root, self, &dummy_parent, None);
    }

    pub fn end_figure_to_string(&self) -> (String, Option<String>) {
        // In svg11 mode the tree is built exactly as for svg, then downgraded
        // to SVG 1.1 at output time. Convert a throwaway copy so the diagram's
        // own tree is left intact.
        let is_svg11 = self.output_format() == "svg11";
        let root = if is_svg11 {
            let copy = xml::deep_copy(&self.root);
            crate::core::svg11::convert(&copy);
            copy
        } else {
            self.root.clone()
        };
        let svg_string = xml::to_string(&root);
        // Python does not write annotations alongside svg11 output.
        let annotation_string = if is_svg11 {
            None
        } else {
            self.annotations_root.as_ref().map(|annotations| {
                let diagram = xml::new_element("diagram");
                xml::append(&diagram, annotations);
                xml::to_string(&diagram)
            })
        };
        (svg_string, annotation_string)
    }

    // ---------- outlining ----------

    pub fn add_outline(
        &mut self,
        element: &El,
        path: &El,
        parent: &El,
        outline_width: Option<i64>,
    ) {
        let outline_width = outline_width.unwrap_or(if self.format == "tactile" { 18 } else { 4 });

        let (stroke, width, fill) = {
            let mut p = path.borrow_mut();
            let stroke = p.pop_attr("stroke").unwrap_or_else(|| "none".to_string());
            let width = p
                .pop_attr("stroke-width")
                .unwrap_or_else(|| "1".to_string());
            let fill = p.pop_attr("fill").unwrap_or_else(|| "none".to_string());
            p.pop_attr("stroke-dasharray");
            (stroke, width, fill)
        };
        let _ = stroke;

        let existing_id = element.borrow().get("id");
        self.add_id(element, existing_id.as_deref());
        let outline_id = format!("{}-outline", element.borrow().get_or("id", "none"));
        path.borrow_mut().set("id", &outline_id);
        self.add_reusable(path);

        let use_el = xml::sub_element(parent, "use");
        {
            let mut u = use_el.borrow_mut();
            u.set("fill", &fill);
            let width_num: i64 = width.parse().unwrap_or(1);
            u.set("stroke-width", &format!("{}", width_num + outline_width));
            u.set("stroke", "white");
            u.set("href", &format!("#{outline_id}"));
        }
        for marker in ["marker-end", "marker-start", "marker-mid"] {
            let reference = path.borrow().get(marker);
            if let Some(reference) = reference {
                use_el
                    .borrow_mut()
                    .set(marker, &reference.replace(')', "-outline)"));
            }
        }
    }

    pub fn finish_outline(
        &mut self,
        element: &El,
        stroke: Option<String>,
        thickness: Option<String>,
        fill: &str,
        parent: &El,
    ) {
        let use_el = xml::sub_element(parent, "use");
        {
            let mut u = use_el.borrow_mut();
            u.set("id", &element.borrow().get_or("id", "none"));
            u.set("fill", fill);
            u.set(
                "stroke-width",
                &thickness.unwrap_or_else(|| "None".to_string()),
            );
            u.set("stroke", &stroke.unwrap_or_else(|| "None".to_string()));
            u.set(
                "stroke-dasharray",
                &element.borrow().get_or("dash", "none"),
            );
        }
        if element.borrow().get_or("id", "none") == parent.borrow().get_or("id", "none") {
            use_el.borrow_mut().pop_attr("id");
        }

        let element_id = element.borrow().get_or("id", "none");
        let suffix = self.id_suffix.last().cloned().unwrap_or_default();
        let reuse_handle = if element_id.ends_with(&suffix) {
            format!("{element_id}-outline")
        } else {
            format!("{element_id}{suffix}-outline")
        };
        use_el
            .borrow_mut()
            .set("href", &format!("#{reuse_handle}"));
        if let Some(reusable) = self.get_reusable(&reuse_handle) {
            for marker in ["marker-start", "marker-end", "marker-mid"] {
                let value = reusable.borrow().get_or(marker, "none");
                if value != "none" {
                    use_el.borrow_mut().set(marker, &value);
                    reusable.borrow_mut().pop_attr(marker);
                }
            }
        }
    }

    // ---------- annotations ----------

    pub fn initialize_annotations(&mut self) {
        if self.annotations_root.is_some() {
            log::error!("Annotations need to be in a single tree");
            return;
        }
        self.annotations_root = Some(xml::new_element("annotations"));
    }

    pub fn add_default_annotation(&mut self, annotation: El) {
        if self.add_default_annotations {
            self.default_annotations.push(annotation);
        }
    }

    pub fn get_default_annotations(&self) -> Vec<El> {
        self.default_annotations.clone()
    }

    pub fn get_annotations_root(&self) -> Option<El> {
        self.annotations_root.clone()
    }

    pub fn add_annotation(&mut self, annotation: &El) {
        if let Some(root) = &self.annotations_root {
            xml::append(root, annotation);
        }
    }

    pub fn push_to_annotation_branch(&mut self, annotation: El) {
        if self.annotation_branch_stack.is_empty() {
            let id = annotation.borrow().get_or("id", "none");
            self.annotation_branches.insert(id, annotation.clone());
        } else {
            self.add_annotation_to_branch(annotation.clone());
        }
        self.annotation_branch_stack.push(annotation);
    }

    pub fn pop_from_annotation_branch(&mut self) {
        self.annotation_branch_stack.pop();
    }

    pub fn add_annotation_to_branch(&mut self, annotation: El) {
        if self.annotation_branch_stack.is_empty() {
            let id = annotation.borrow().get_or("id", "none");
            let id = self.prepend_id_prefix(&id);
            self.annotation_branches.insert(id, annotation);
            return;
        }
        let branch = self
            .annotation_branch_stack
            .last()
            .expect("stack is non-empty")
            .clone();
        xml::append(&branch, &annotation);
        let id = self.append_id_suffix(&annotation);
        let id = self.prepend_id_prefix(&id);
        annotation.borrow_mut().set("id", &id);
    }

    pub fn get_annotation_branch(&mut self, id: &str) -> Option<El> {
        self.annotation_branches.shift_remove(id)
    }

    // ---------- textures ----------

    pub fn add_texture(&mut self, texture: &str, color: &str) -> String {
        let tactile = self.format == "tactile";
        let color = if tactile { "#777" } else { color };
        let clean_color = repeat::epub_clean(color);
        let texture_str = format!("{texture}-{color}");
        if let Some(id) = self.textures.get(&texture_str) {
            return id.clone();
        }

        let pattern = xml::sub_element(&self.defs, "pattern");
        {
            let mut p = pattern.borrow_mut();
            p.set("x", "0");
            p.set("y", "0");
            p.set("patternUnits", "userSpaceOnUse");
        }

        let mk_line = |x1: &str, y1: &str, x2: &str, y2: &str| {
            let line = xml::sub_element(&pattern, "line");
            let mut l = line.borrow_mut();
            l.set("x1", x1);
            l.set("y1", y1);
            l.set("x2", x2);
            l.set("y2", y2);
            l.set("stroke", color);
        };

        let id = match texture {
            "horizontal" => {
                let id = self.prepend_id_prefix(&format!("__horizontal_texture_{clean_color}"));
                pattern.borrow_mut().set("id", &id);
                let (s, thickness) = if tactile { (9, "1") } else { (7, "2") };
                pattern.borrow_mut().set("width", &s.to_string());
                pattern.borrow_mut().set("height", &s.to_string());
                mk_line("-1", "0", &(s + 1).to_string(), "0");
                pattern.borrow().children[0]
                    .borrow_mut()
                    .set("stroke-width", thickness);
                id
            }
            "vertical" => {
                let id = self.prepend_id_prefix(&format!("__vertical_texture_{clean_color}"));
                pattern.borrow_mut().set("id", &id);
                let (s, thickness) = if tactile { (9, "1") } else { (7, "2") };
                pattern.borrow_mut().set("width", &s.to_string());
                pattern.borrow_mut().set("height", &s.to_string());
                mk_line("0", "-1", "0", &(s + 1).to_string());
                pattern.borrow().children[0]
                    .borrow_mut()
                    .set("stroke-width", thickness);
                id
            }
            "diagonal" | "backdiagonal" => {
                let id =
                    self.prepend_id_prefix(&format!("__{texture}_texture_{clean_color}"));
                pattern.borrow_mut().set("id", &id);
                let s = if tactile { 13 } else { 9 };
                pattern.borrow_mut().set("width", &s.to_string());
                pattern.borrow_mut().set("height", &s.to_string());
                let rows: [[i64; 4]; 3] = if texture == "diagonal" {
                    [[-1, 1, 1, -1], [-1, s + 1, s + 1, -1], [s - 1, s + 1, s + 1, s - 1]]
                } else {
                    [[s - 1, -1, s + 1, 1], [-1, -1, s + 1, s + 1], [-1, s - 1, 1, s + 1]]
                };
                for data in rows {
                    mk_line(
                        &data[0].to_string(),
                        &data[1].to_string(),
                        &data[2].to_string(),
                        &data[3].to_string(),
                    );
                }
                for line in &pattern.borrow().children {
                    line.borrow_mut().set("stroke-width", "1");
                }
                id
            }
            "dot" => {
                let id = self.prepend_id_prefix(&format!("__dot_texture_{clean_color}"));
                pattern.borrow_mut().set("id", &id);
                let (s, dot_size) = if tactile { (9.0, 2.0) } else { (8.0, 1.8) };
                let s3 = 3f64.sqrt();
                let r = 2.0 / s3 * s;
                let t = 1.0 / s3 * s;
                pattern.borrow_mut().set("width", &py_str(2.0 * s));
                pattern.borrow_mut().set("height", &py_str(2.0 * s3 * s));
                for center in [
                    [0.0, 0.0],
                    [0.0, r],
                    [0.0, 2.0 * r],
                    [s, t],
                    [s, 2.0 * r + t],
                    [s, r + t],
                ] {
                    let circle = xml::sub_element(&pattern, "circle");
                    let mut c = circle.borrow_mut();
                    c.set("cx", &py_str(center[0] + dot_size));
                    c.set("cy", &py_str(center[1] + dot_size));
                    c.set("r", &py_str(dot_size));
                    c.set("fill", color);
                }
                id
            }
            "diamond" => {
                let id = self.prepend_id_prefix(&format!("__diamond_texture_{clean_color}"));
                pattern.borrow_mut().set("id", &id);
                let s = if tactile { 5.0 } else { 2.5 };
                let t = s * 3f64.sqrt();
                pattern.borrow_mut().set("width", &py_str(4.0 * s));
                pattern.borrow_mut().set("height", &py_str(4.0 * t));
                for (cx, cy) in [(s, t), (3.0 * s, 3.0 * t)] {
                    let path = xml::sub_element(&pattern, "path");
                    let mut p = path.borrow_mut();
                    p.set(
                        "d",
                        &format!(
                            "M {} {} L {} {} L {} {} L {} {} Z",
                            py_str(cx),
                            py_str(cy + t),
                            py_str(cx + s),
                            py_str(cy),
                            py_str(cx),
                            py_str(cy - t),
                            py_str(cx - s),
                            py_str(cy)
                        ),
                    );
                    p.set("fill", color);
                }
                id
            }
            _ => String::new(),
        };

        self.textures.insert(texture_str, id.clone());
        id
    }
}
