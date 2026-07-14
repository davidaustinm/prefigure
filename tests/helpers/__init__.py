"""Shared helpers for the PreFigure Python test suite.

These modules are the Python twins of the Rust port's test harness:
``compare`` mirrors ``rust/prefig-core/tests/svg_compare.rs`` and ``build_helper``
mirrors the ``build_one`` routine used to generate the golden SVGs. They read the
same neutral assets under ``tests/`` that the Rust tests will later be re-pointed
at, so both languages check the same corpus.
"""
