from proto_bytemessage_py.proto_bytemessage_py import (
    PyMessageBatch,
)  # This comes from your Rust bindings


class Processor:
    """Base class for Python processors."""

    def process(self, input_batch: PyMessageBatch) -> PyMessageBatch:
        """Process a message batch and return a new message batch."""
        raise NotImplementedError

    def process_message(self, msg_type: int, payload: bytes) -> list[tuple[int, bytes]]:
        """Process a single message. Override this for simpler processors.

        Returns a list of (msg_type, payload) tuples for the output batch.
        """
        raise NotImplementedError

    def process_messages(
        self,
        messages: list[tuple[int, bytes]],
    ) -> list[tuple[int, bytes]]:
        """Process multiple messages. Override this for batch processing.

        Returns a list of (msg_type, payload) tuples for the output batch.
        """
        return [
            result
            for msg_type, payload in messages
            for result in self.process_message(msg_type, payload)
        ]
