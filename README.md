# zeroq

[zeroq](https://github.com/idkosilov/zeroq) is a high-performance, shared-memory, 
multi-producer/multi-consumer (MPMC) queue written in Rust with Python bindings. 
Designed for concurrent and inter-process communication, zeroq delivers 
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
pip install zeroq
```

## Benchmarks

The following benchmarks compare `zeroq` with Python's standard 
`multiprocessing.Queue` across different payload sizes. Tests were conducted 
on a **GitHub Actions runner (Ubuntu latest)** with Python 3.11. Each test measures 
the time for a complete `put`+`get` operation cycle (1000 iterations).

![benchmarks](https://raw.githubusercontent.com/idkosilov/ZeroQ/refs/heads/main/benchmark_plot.png)

### Observations

zeroq significantly accelerates interprocess communication, 
delivering up to 50× faster data transfer compared to `multiprocessing.Queue`. 
This performance gain is achieved through shared memory and lock-free 
synchronization, eliminating the overhead of serialization and dynamic memory 
allocation.

### Optimal Use Cases

zeroq is particularly well-suited for tasks that require fast, 
low-latency data exchange between processes, including:

- **ML/AI Pipelines:** Efficient data transfer between processes for image and video processing, such as inference in computer vision models.
- **Multimedia Processing:** Passing video frames or audio streams between processes in real-time applications.
- **Real-Time Systems:** Handling events, logs, telemetry, and signals where minimal latency and high throughput are critical.
- **IoT and Embedded Systems:** Fast transmission of fixed-size data blocks between processes on a single device.

By leveraging shared memory and avoiding unnecessary memory operations, 
zeroq provides a significant advantage in applications that demand 
high-speed, low-latency communication.

## Usage

Once installed, you can easily integrate zeroq into your Python projects. Below is an example of video player.

To run this example, install OpenCV and FFmpeg:

```bash
pip install opencv-python
```

Make sure FFmpeg is installed and accessible via the command line. You can download it from ffmpeg.org and add it to your system’s PATH.


```python
import multiprocessing
import subprocess

import cv2
import numpy as np

from zeroq import Empty, Full, Queue


def producer(video_path: str) -> None:
    """Reads video frames via FFmpeg and pushes them to the shared queue."""
    queue = Queue(
        name='video-queue',
        element_size=1920 * 1080 * 3,
        capacity=16,
        create=True,
    )

    process = subprocess.Popen(
        [
            'ffmpeg', '-re', '-i', video_path,
            '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-vf', 'scale=1920:1080',
            '-vcodec', 'rawvideo', '-an', '-nostdin', 'pipe:1',
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        bufsize=1920 * 1080 * 3,
    )

    while True:
        raw_frame = process.stdout.read(1920 * 1080 * 3)
        if not raw_frame:
            break

        try:
            queue.put(raw_frame, timeout=1.0)
        except Full:
            break


def consumer():
    """Retrieves frames from the queue and displays them using OpenCV."""
    queue = Queue(name='video-queue', create=False)

    while True:
        try:
            frame_data = queue.get(timeout=1.0)
        except Empty:
            break

        frame = (
            np.frombuffer(frame_data, dtype=np.uint8)
            .reshape((1080, 1920, 3))
        )
        cv2.imshow('Video Stream', frame)

        if cv2.waitKey(1) & 0xFF == ord('q'):
            break

    cv2.destroyAllWindows()


if __name__ == '__main__':
    video_path = 'your_video.mp4'

    multiprocessing.Process(target=producer, args=(video_path,)).start()
    multiprocessing.Process(target=consumer).start()
```


## License

zeroq is distributed under the terms of the MIT License.
