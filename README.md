# fastqueue

[fastqueue](https://github.com/idkosilov/fastqueue) is a high-performance, shared-memory, multi-producer/multi-consumer (MPMC) queue written in Rust with Python bindings. Designed for concurrent and inter-process communication, fastqueue delivers low latency and strict FIFO (First-In, First-Out) behavior even under heavy load.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
  - [Pre-built Wheels](#pre-built-wheels)
  - [Building from Source](#building-from-source)
- [Usage](#usage)
- [License](#license)

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

## License

fastqueue is distributed under the terms of the MIT License.


