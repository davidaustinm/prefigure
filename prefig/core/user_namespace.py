import ast
import math
import logging
import numpy as np
from . import math_utilities
from . import calculus
from math import *
from .math_utilities import *

# Allow authors to perform some mathematical operations and to define
# some quantities in a safe way.  This essentially defines a namespace
# available to authors.  We use an abstract syntax tree (AST) to
# parse an author-generated expression and check that it only uses
# safe python operations.  We also replace any lists or tuples with
# equivalent numpy arrays

logger = logging.getLogger('prefigure')

inf = np.inf

# Record built-in python functions and constants as allowed
functions = {x for x in dir(math) + dir(math_utilities) if not "__" in x}.difference({'e', 'pi'})
variables = {'e', 'pi', 'inf'}

# Transforms an AST by wrapping any List or Tuple inside a numpy array
class TransformList(ast.NodeTransformer):
    def visit_Tuple(self, node):
        return self.visit_List(node)
    def visit_List(self, node):
        if isinstance(node, (ast.List, ast.Tuple)):
            # descend the ast and visit each of the children
            node.elts = [self.visit(elt) for elt in node.elts]
            # now wrap a tuple or list in a numpy array
            return ast.Call(
                func=ast.Attribute(
                    value=ast.Name(id='np', ctx=ast.Load()),
                    attr='array',
                    ctx=ast.Load()),
                args=[node],
                keywords=[])

        else:
            return node

# Evaluate a safe expression after transforming the AST as above
def transform_eval(expr):
    tree = ast.parse(expr, mode='eval')
    transformed_tree = TransformList().visit(tree)
    ast.fix_missing_locations(transformed_tree)
    try:
        return eval(compile(transformed_tree, '', 'eval'))
    except ValueError:
        # there is an inhomogeneous numpy array
        # we've validated already so we'll just evaluate the original expr
        return eval(expr)

# validate an individual node inside an AST.  These are the allowed
# python constructions.  This function will be called recursively on
# each node of the tree
def validate_node(node, args=None):
    if isinstance(node, ast.Constant):
        return True
    if isinstance(node, ast.Expression):
        return validate_node(node.body, args)
    if isinstance(node, ast.Name):
        if node.id in variables:
            return True
        if args is not None and node.id in args:
            return True
        raise SyntaxError(f"Unrecognized name: {node.id}")
    if isinstance(node, (ast.List, ast.Tuple)):
        return all([validate_node(elt, args) for elt in node.elts])
    if isinstance(node, ast.Dict):
        for key in node.keys:
            if not validate_node(key):
                logger.error(f'Illegal key in dictionary: {key}')
                return False
        for value in node.values:
            if not validate_node(value):
                logger.error(f'Illegal value in dictionary: {value}')
                return False
        return True
    if isinstance(node, ast.BinOp):
        return validate_node(node.left, args) and validate_node(node.right, args)
    if isinstance(node, ast.UnaryOp):
        return validate_node(node.operand, args)
    if isinstance(node, ast.Subscript):
        return validate_node(node.value, args) and validate_node(node.slice, args)
    if isinstance(node, ast.Index):
        return validate_node(node.value)
    if isinstance(node, ast.Call):
        if node.func.id in functions:
            return all([validate_node(arg, args) for arg in node.args])
        logger.error(f"Unknown function in evaluation: {node.func.id}")
        return False
    logger.error(f"Unrecognized construction in evaluation")
    return False

# Validate an expression by sending the AST's root to validate_node
# and traversing the tree recursively
def validate(s, args=None):
    tree = ast.parse(s, mode='eval')
    return validate_node(tree, args)

# Validate and then evaluate a valid expression.  This function
# will be called from other parts of the project.
def valid_eval(s, name=None, substitution=True):
    if s is None:
        logger.error(f"Evaluating an empty object.")
        raise SyntaxError(f'Evaluating an empty object.  Perhaps there is a required attribute that is missing')
    if substitution:
        s = s.replace('^', '**')
    equal = s.find('=')
    if s.strip()[0] == r'#':  # it's a color
        return s
    if s.strip().startswith('rgb'): # it's a color
        return s
    # is this a function?  If so:
    if equal >= 0:
        name, expr = [field.strip() for field in s.split('=')]
        open = name.find('(')
        close = name.find(')')
        args = name[open+1: close].strip()
        name = name[:open]
        if validate(expr, args):
            cmd = 'lambda ' + args + ': ' + expr
            functions.add(name)
            variables.add(name)
            globals()[name] = transform_eval(cmd)
            return globals()[name]
        else:
            logger.error(f"Unsafe function definition: {expr}")
            raise SyntaxError(f'Unsafe function definition: {expr}')
        return
    # otherwise, it's just an expression
    else:
        if validate(s):
            value = transform_eval(s)
            if name is not None:
                variables.add(name)
                globals()[name] = value
            return value
        else:
            logger.error(f"Unsafe definition: {s}")
            raise SyntaxError(f'Unsafe definition: {s}')

# used in a definition tag
def define(expression, substitution=True):
    try:
        left, right = [side.strip() for side in expression.split("=")]
    except:
        logger.error(f"Unrecognized definition: {expression}")
        return
    if left.find('(') > 0:
        valid_eval(expression, substitution=substitution)
    else:
        valid_eval(right, left, substitution=substitution)

# retrieves and evaluates an author-defined function
def evaluate(function, a):
    return globals()[function](a)

# retrieves a one-variable function and returns its derivative
def derivative(f, name):
    globals()[name] = lambda x: calculus.derivative(f, x)
    functions.add(name)
    variables.add(name)

def enter_namespace(name, value):
    globals()[name] = value
    variables.add(name)
