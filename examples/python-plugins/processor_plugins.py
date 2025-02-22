from typing import Any

from plugin_python_service import CoProcessorBase, DrivingProcessorBase


class PythonNumberDoubler(CoProcessorBase):
    def __init__(self):
        super().__init__()
        self.init(
            name="PythonNumberDoubler",
            description="Doubles each number in the input (Python implementation)",
        )

    def process(self, input_data: dict[str, Any]) -> dict[str, Any]:
        numbers = input_data.get("numbers", [])
        return {"numbers": [n * 2 for n in numbers]}


class PythonBatchAccumulator(DrivingProcessorBase):
    def __init__(self):
        super().__init__()
        self.init(
            name="PythonBatchAccumulator",
            description="Accumulates and processes numbers in batches (Python implementation)",
        )
        self.current_batch: list[int] = []
        self.batch_size = 3

    def start(self, input_data: dict[str, Any]) -> tuple[str, dict[str, Any]]:
        self.current_batch = []
        numbers = input_data.get("numbers", [])

        if not numbers:
            return "complete", {"numbers": []}

        # Send first batch
        batch = numbers[: self.batch_size]
        self.current_batch.extend(numbers[self.batch_size :])
        return "needs_coprocessing", {"numbers": batch}

    def continue_processing(
        self,
        coprocessor_result: dict[str, Any],
    ) -> tuple[str, dict[str, Any]]:
        processed_numbers = coprocessor_result.get("numbers", [])
        result_numbers = []

        # Process complete batches
        if processed_numbers:
            result_numbers.extend([sum(processed_numbers)])

        # If we have more numbers to process
        if self.current_batch:
            next_batch = self.current_batch[: self.batch_size]
            self.current_batch = self.current_batch[self.batch_size :]
            return "needs_coprocessing", {"numbers": next_batch}

        # If we're done
        return "complete", {"numbers": result_numbers}
