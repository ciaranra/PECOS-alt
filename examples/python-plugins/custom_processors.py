from plugin_python_service import CoProcessorBase


class CustomNumberFilter(CoProcessorBase):
    def __init__(self):
        super().__init__()
        self.init(
            name="CustomNumberFilter",
            description="Filters numbers based on a predicate",
        )

    def process(self, input_data):
        numbers = input_data.get("numbers", [])
        # Example: filter even numbers
        filtered = [n for n in numbers if n % 2 == 0]
        return {"numbers": filtered}
