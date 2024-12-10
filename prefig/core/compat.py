
class ErrorOnAccess:
    """Class to provide runtime errors for libraries that cannot be imported.
    This class will error when any property is accessed."""
    def __init__(self, import_name):
        self.import_name = import_name

    def __getattr__(self, name):
        if name == 'import_name':
            return super().__getattribute__(name)
        raise AttributeError(f"Library '{self.import_name}' failed to load; cannot access '{self.import_name}.{name}'")

    def __setattr__(self, name, value):
        if name == 'import_name':
            super().__setattr__(name, value)
            return
        raise AttributeError(f"Library '{self.import_name}' failed to load; cannot set '{self.import_name}.{name}'")