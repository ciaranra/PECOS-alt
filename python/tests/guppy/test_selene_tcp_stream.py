"""Test accessing Selene's TCP result stream directly.

This explores how to tap into the TCP stream that Selene uses to communicate
results, which is essential for extracting final results in our integration.
"""

import socket
import tempfile
import threading
from pathlib import Path

try:
    import selene_sim
    from selene_sim.backends import Coinflip, SimpleRuntime
    from selene_sim.result_handling import ResultStream, TCPStream

    SELENE_AVAILABLE = True
except ImportError:
    SELENE_AVAILABLE = False


def test_tcp_stream_directly() -> None:
    """Test creating and using a TCPStream directly."""
    if not SELENE_AVAILABLE:
        print("Selene not available")
        return

    print("=" * 60)
    print("TESTING SELENE TCP STREAM")
    print("=" * 60)

    # Create a TCP stream
    print("\n1. Creating TCPStream...")

    # TCPStream is a context manager that sets up a TCP server
    with TCPStream(
        host="localhost",
        port=0,  # Let system choose port
        logfile=None,
        shot_offset=0,
        shot_increment=1,
    ) as stream:
        print(f"   Created stream: {stream}")
        print(f"   URI: {stream.get_uri()}")

        # The URI tells Selene where to connect
        uri = stream.get_uri()

        # Parse the URI to understand the format
        # It's typically something like "tcp://127.0.0.1:PORT"
        print(f"   Connection info: {uri}")

        # In a real scenario, Selene would connect to this URI
        # Let's simulate what would happen

        # Create a client connection to test
        if uri.startswith("tcp://"):
            host_port = uri[6:]  # Remove "tcp://"
            host, port = host_port.split(":")
            port = int(port)

            print(f"\n2. Simulating client connection to {host}:{port}")

            # Connect in a separate thread
            def client_thread() -> None:
                try:
                    client_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                    client_socket.connect((host, port))
                    print("   Client connected!")

                    # Send some test data (simulating Selene output)
                    # Selene sends encoded messages
                    test_message = b"TEST:DATA:example\x00"
                    client_socket.send(test_message)
                    print(f"   Sent: {test_message}")

                    client_socket.close()
                except Exception as e:
                    print(f"   Client error: {e}")

            client = threading.Thread(target=client_thread)
            client.start()

            # Try to read from the stream
            print("\n3. Reading from stream...")
            try:
                # TCPStream has methods to read data
                # Check what methods are available
                print(
                    f"   Stream methods: {[m for m in dir(stream) if not m.startswith('_')]}",
                )

                # Wait for client to connect
                client.join(timeout=1)

            except Exception as e:
                print(f"   Read error: {e}")


def test_result_stream_wrapper() -> None:
    """Test the ResultStream wrapper around TCPStream."""
    if not SELENE_AVAILABLE:
        print("Selene not available")
        return

    print("\n" + "=" * 60)
    print("TESTING RESULT STREAM")
    print("=" * 60)

    # ResultStream wraps TCPStream and provides iteration
    with TCPStream() as tcp_stream:
        result_stream = ResultStream(tcp_stream)

        print(f"1. Created ResultStream: {result_stream}")
        print(f"   Methods: {[m for m in dir(result_stream) if not m.startswith('_')]}")

        # ResultStream is an iterator that yields results
        # In practice, Selene would be writing to the TCP stream
        # and we would iterate over ResultStream to read them


def test_selene_with_custom_stream() -> None:
    """Test running Selene with a custom output stream we control."""
    if not SELENE_AVAILABLE:
        print("Selene not available")
        return

    print("\n" + "=" * 60)
    print("TESTING CUSTOM STREAM WITH SELENE")
    print("=" * 60)

    # Create a simple LLVM program
    llvm_ir = """
    declare void @__quantum__qis__h__body(i64)
    declare i1 @__quantum__qis__mz__body(i64)
    declare void @__quantum__rt__result_record(i8*, i1)

    define void @main() {
        call void @__quantum__qis__h__body(i64 0)
        %m = call i1 @__quantum__qis__mz__body(i64 0)

        ; This should appear in the result stream
        %tag = bitcast [7 x i8]* @result_tag to i8*
        call void @__quantum__rt__result_record(i8* %tag, i1 %m)
        ret void
    }

    @result_tag = private constant [7 x i8] c"output\\00"
    """

    with tempfile.TemporaryDirectory() as tmpdir:
        # Save LLVM
        llvm_file = Path(tmpdir) / "test.ll"
        llvm_file.write_text(llvm_ir)

        try:
            # Try to build with Selene
            # We need to understand how to intercept the output_stream

            # When Selene builds, it can accept output_stream configuration
            # Let's see if we can provide our own

            print("1. Attempting to build with custom stream...")

            # The build function might accept output_stream parameter
            # or it might be configured in the SeleneInstance

            # Let's check SeleneInstance.run_shots parameters
            import inspect

            from selene_sim import SeleneInstance

            sig = inspect.signature(SeleneInstance.run_shots)
            print(f"   run_shots signature: {sig}")

        except Exception as e:
            print(f"   Error: {e}")


def test_intercepting_results() -> None:
    """Document how to intercept results from Selene."""
    print("\n" + "=" * 60)
    print("INTERCEPTING SELENE RESULTS")
    print("=" * 60)

    print(
        """
APPROACH 1: Custom TCP Stream
- Create our own TCPStream before running Selene
- Pass the URI to Selene's configuration
- Read from the stream while Selene executes
- Parse the tagged results

APPROACH 2: Hook into parse_shot
- Selene's parse_shot yields results as it processes
- We can iterate over these results
- Each shot boundary marks completion

APPROACH 3: Unparsed Mode
- Set parse_results=False in run_shots()
- Get raw ("USER:TYPE:tag", value) tuples
- Process them ourselves

APPROACH 4: Direct Socket Connection
- Connect to Selene's TCP stream as a client
- Read the raw bytes
- Decode the messages ourselves

For SeleneExecutableEngine integration:
1. Start TCPStream in the engine
2. Configure Selene executable with the URI
3. Run the executable
4. Read results from the stream
5. Convert to PECOS Shot format
""",
    )


def explore_selene_instance_internals() -> None:
    """Look at how SeleneInstance handles streams internally."""
    if not SELENE_AVAILABLE:
        return

    print("\n" + "=" * 60)
    print("SELENE INSTANCE INTERNALS")
    print("=" * 60)

    # Create a minimal SeleneInstance to explore
    with tempfile.TemporaryDirectory() as tmpdir:
        from selene_sim import SeleneInstance

        # Create instance
        instance = SeleneInstance(
            root=Path(tmpdir),
            artifacts=Path(tmpdir) / "artifacts",
            runs=Path(tmpdir) / "runs",
            executable=Path("/bin/true"),  # Dummy executable
            library_search_dirs=[],
        )

        print("1. SeleneInstance attributes:")
        for attr in ["root", "artifacts", "runs", "executable"]:
            print(f"   {attr}: {getattr(instance, attr)}")

        # Check the run_shots method to understand configuration
        print("\n2. How run_shots configures output:")

        # The key is in how run_shots creates the global_configuration
        # It sets: global_configuration["output_stream"] = data_stream.get_uri()
        # This tells the Selene executable where to send results

        print(
            """
   In run_shots():
   - Creates TCPStream with context manager
   - Sets config["output_stream"] = stream.get_uri()
   - Passes config to Selene executable
   - Reads from stream while executable runs

   This means we can:
   1. Create our own TCPStream
   2. Pass its URI to the Selene executable
   3. Read results directly from the stream
   """,
        )


if __name__ == "__main__":
    print("SELENE TCP STREAM EXPLORATION")
    print("=" * 60)

    test_tcp_stream_directly()
    test_result_stream_wrapper()
    test_selene_with_custom_stream()
    test_intercepting_results()
    explore_selene_instance_internals()
