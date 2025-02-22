
# Python Examples

Examples of Python plugins for the plugin system.

## Current Examples
- `custom_processors.py` - Shows how to create custom processing plugins
- `processor_plugins.py` - Demonstrates number doubling and batch accumulation

## Creating a New Plugin

1. Create a new Python file in this directory

2. Import the base classes:
```python
from python_service import CoProcessorBase, DrivingProcessorBase
```

3. Create your processor class:
```python
class MyProcessor(CoProcessorBase):
    def __init__(self):
        super().__init__()
        self.init(
            name="MyProcessor",
            description="Description of what your processor does"
        )

    def process(self, input_data):
        # Your processing logic here
        numbers = input_data.get("numbers", [])
        # Do something with numbers
        return {"numbers": processed_numbers}
```

4. For driving processors:
```python
class MyDrivingProcessor(DrivingProcessorBase):
    def __init__(self):
        super().__init__()
        self.init(
            name="MyDrivingProcessor",
            description="Description of your driving processor"
        )
        self.state = []  # Any state you need

    def start(self, input_data):
        # Initialize processing
        return "needs_coprocessing", {"data": initial_data}

    def continue_processing(self, coprocessor_result):
        # Handle coprocessor results
        if done:
            return "complete", {"result": final_result}
        return "needs_coprocessing", {"data": next_batch}
```

## Notes
- Python plugins are automatically discovered from `.py` files in this directory
- Plugins must subclass either `CoProcessorBase` or `DrivingProcessorBase`
- All data exchange is done through JSON-serializable dictionaries
