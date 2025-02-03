# fastqueue

[fastqueue](https://github.com/idkosilov/fastqueue) is a high-performance, shared-memory, 
multi-producer/multi-consumer (MPMC) queue written in Rust with Python bindings. 
Designed for concurrent and inter-process communication, fastqueue delivers 
low latency and strict FIFO (First-In, First-Out) behavior even under heavy load.

## Features

- **Speed:** Up to 50× faster data transfer compared to `multiprocessing.Queue`.
- **Lock-Free Concurrency:** Utilizes atomic operations for efficient, lock-free synchronization across threads and processes.
- **Shared Memory Communication:** Enables fast inter-process messaging without the overhead of kernel-based IPC.
- **Flexible API:** Supports both blocking (`put`/`get`) and non-blocking (`put_nowait`/`get_nowait`) operations.
- **Predictable FIFO Ordering:** Guarantees that elements are dequeued in the exact order they were enqueued.
- **Python Bindings:** Easily integrate with Python projects while leveraging Rust's performance and safety.

## Installation

If pre-built wheels are available on [PyPI](https://pypi.org), installation is as simple as:

```bash
pip install fastqueue
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

- **ML/AI Pipelines:** Efficient data transfer between processes for image and video processing, such as inference in computer vision models.
- **Multimedia Processing:** Passing video frames or audio streams between processes in real-time applications.
- **Real-Time Systems:** Handling events, logs, telemetry, and signals where minimal latency and high throughput are critical.
- **IoT and Embedded Systems:** Fast transmission of fixed-size data blocks between processes on a single device.

By leveraging shared memory and avoiding unnecessary memory operations, 
Fastqueue provides a significant advantage in applications that demand 
high-speed, low-latency communication.

## Usage

Once installed, you can easily integrate fastqueue into your Python projects. Below is a basic usage example:

```python
from fastqueue import Queue, Empty, Full
import multiprocessing
import time

QUEUE_NAME = "shared-queue"
ELEMENT_SIZE = 32  # Fixed size for each message (bytes)
CAPACITY = 1024  # Must be a power of two

def producer():
    """ Producer process that enqueues data into the shared queue. """
    queue = Queue(name=QUEUE_NAME, element_size=ELEMENT_SIZE, capacity=CAPACITY, create=True)

    for i in range(10):
        data = f"message_{i}".encode()  # Convert string to bytes
        try:
            queue.put(data)
            print(f"[Producer] Produced: {data.decode()}")
        except Full:
            print("[Producer] Queue is full! Waiting before retrying...")
            time.sleep(0.1)

def consumer():
    """ Consumer process that dequeues data from the shared queue. """
    queue = Queue(name=QUEUE_NAME, element_size=ELEMENT_SIZE, capacity=CAPACITY, create=False)

    while True:
        try:
            item = queue.get()
            print(f"[Consumer] Consumed: {item.decode()}")
        except Empty:
            print("[Consumer] Queue is empty! Consumer exiting...")
            break

if __name__ == "__main__":
    producer_process = multiprocessing.Process(target=producer)
    consumer_process = multiprocessing.Process(target=consumer)

    producer_process.start()
    consumer_process.start()

    producer_process.join()
    consumer_process.join()
```


## License

fastqueue is distributed under the terms of the MIT License.


