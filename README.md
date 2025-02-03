# fastqueue

[fastqueue](https://github.com/idkosilov/fastqueue) is a high-performance, shared-memory, 
multi-producer/multi-consumer (MPMC) queue written in Rust with Python bindings. 
Designed for concurrent and inter-process communication, fastqueue delivers 
low latency and strict FIFO (First-In, First-Out) behavior even under heavy load.

## Features

- **Lock-Free Concurrency:** Utilizes atomic operations for efficient, lock-free synchronization across threads and processes.
- **Shared Memory Communication:** Enables fast inter-process messaging without the overhead of kernel-based IPC.
- **Flexible API:** Supports both blocking (`put`/`get`) and non-blocking (`put_nowait`/`get_nowait`) operations.
- **Predictable FIFO Ordering:** Guarantees that elements are dequeued in the exact order they were enqueued.
- **Python Bindings:** Easily integrate with Python projects while leveraging Rust's performance and safety.

## Installation

### Pre-built Wheels

If pre-built wheels are available on [PyPI](https://pypi.org), installation is as simple as:

```bash
pip install fastqueue
```

### Building from Source

If you prefer to build the package yourself, follow these instructions. Before you begin, ensure that you have the following prerequisites:
- Rust (with Cargo): Required for building the core library.
- Python 3.9+ : To run the Python bindings.
- Maturin: Recommended tool for building and publishing Rust-powered Python packages.
- C Compiler (e.g., GCC or Clang): Needed to build Rust dependencies.

#### Clone the Repository

```bash
git clone https://github.com/idkosilov/fastqueue.git
cd fastqueue
```

#### Build the Package

Use Maturin to build the package in release mode:

```bash
maturin build --release
```

#### Install the Built Package

After building, install the generated wheel (located in the target/wheels/ directory):

```
pip install target/wheels/fastqueue-*.whl
```

## Usage

Once installed, you can easily integrate fastqueue into your Python projects. Below is a basic usage example:

```python
from fastqueue import Queue, Empty, Full

# Create a new queue.
# - 'element_size' specifies the size (in bytes) of each element.
# - 'capacity' must be a power of two.
# - 'create=True' indicates that a new shared memory segment should be created.
queue = Queue(name="my-shm-queue", element_size=32, capacity=1024, create=True)

# Blocking enqueue.
try:
    queue.put(b"example_data")
except Full:
    print("Queue is full!")

# Blocking dequeue with a timeout of 1 second.
try:
    item = queue.get(timeout=1.0)
    print("Dequeued item:", item)
except Empty:
    print("Queue is empty!")

# Non-blocking operations.
try:
    queue.put_nowait(b"another_item")
except Full:
    print("Queue is full (non-blocking)!")

try:
    item = queue.get_nowait()
    print("Dequeued item (non-blocking):", item)
except Empty:
    print("Queue is empty (non-blocking)!")
```

## Benchmarks

![benchmarks](tests/benchmarks/benchmark_plot.png)

The following benchmarks compare `fastqueue` with Python's standard 
`multiprocessing.Queue` across different payload sizes. Tests were conducted 
on an **Apple M1 Pro** CPU with Python 3.11.10. Each test measures 
the time for a complete `put`+`get` operation cycle (1000 iterations). 

| Payload size | Queue Type  | Mean Time per Op | Ops/sec | Speedup |
|--------------|-------------|------------------|---------|---------|
| 8 B          | `fastqueue` | 235.6 ns         | 4.25M   | 50x     |
|              | `mp.Queue`  | 11.84 μs         | 84.4K   |         |
| 128 B        | `fastqueue` | 252.8 ns         | 3.96M   | 47x     |
|              | `mp.Queue`  | 11.86 μs         | 84.3K   |         |
| 1 KiB        | `fastqueue` | 345.4 ns         | 2.90M   | 37x     |
|              | `mp.Queue`  | 12.89 μs         | 77.6K   |         |
| 256 KiB      | `fastqueue` | 23.5 μs          | 42.5K   | 6x      |
|              | `mp.Queue`  | 142.0 μs         | 7.04K   |         |
| 8 MiB        | `fastqueue` | 1.07 ms          | 936     | 13.7x   |
|              | `mp.Queue`  | 14.60 ms         | 68.5    |         |
| 32 MiB       | `fastqueue` | 6.03 ms          | 166     | 7.7x    |
|              | `mp.Queue`  | 46.31 ms         | 22      |         |

Fastqueue significantly accelerates interprocess communication, 
delivering up to 50× faster data transfer compared to `multiprocessing.Queue`. 
This performance gain is achieved through shared memory and lock-free 
synchronization, eliminating the overhead of serialization and dynamic memory 
allocation.

### Optimal Use Cases

Fastqueue is particularly well-suited for tasks that require fast, 
low-latency data exchange between processes, including:

**ML/AI Pipelines:** Efficient data transfer between processes for image and video processing, such as inference in computer vision models.
**Multimedia Processing:** Passing video frames or audio streams between processes in real-time applications.
**Real-Time Systems:** Handling events, logs, telemetry, and signals where minimal latency and high throughput are critical.
**IoT and Embedded Systems:** Fast transmission of fixed-size data blocks between processes on a single device.

By leveraging shared memory and avoiding unnecessary memory operations, 
Fastqueue provides a significant advantage in applications that demand 
high-speed, low-latency communication.

## License

fastqueue is distributed under the terms of the MIT License.


