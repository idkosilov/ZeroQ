class InvalidParameters(Exception):
    """Raised when invalid parameters are passed to the constructor."""


class QueueEmpty(Exception):
    """Raised when the queue is empty."""


class QueueFull(Exception):
    """Raised when the queue is full."""


class QueueClosed(Exception):
    """Raised when the queue is closed."""


class FailedCreateSharedMemory(Exception):
    """Raised when shared memory creation fails."""


class FailedOpenSharedMemory(Exception):
    """Raised when opening an existing shared memory fails."""


class Queue:
    """A shared-memory MPMC queue."""

    def __init__(
        self,
        name: str,
        element_size: int | None = None,
        capacity: int | None = None,
        create: bool = True
    ) -> None:
        """
        Creates a new shared-memory queue or attaches to an existing one.

        :param name: Identifier for the shared memory segment.
        :param element_size: Size of each element in bytes (required if creating).
        :param capacity: Number of slots (must be a power of two; required if creating).
        :param create: Whether to create a new queue (default=True).
        :raises InvalidParameters: If element_size or capacity is missing when creating.
        :raises FailedCreateSharedMemory: If shared memory creation fails.
        :raises FailedOpenSharedMemory: If opening an existing shared memory fails.
        """

    def put(self, item: bytes, timeout: float | None = None) -> None:
        """
        Blocking put operation. Attempts to enqueue `item` into the queue.
        If the queue is full, it blocks until space becomes available or the timeout expires.

        :param item: The item to enqueue.
        :param timeout: Maximum time to wait (seconds), or None for indefinite wait.
        :raises QueueFull: If the queue remains full beyond the timeout.
        """

    def put_nowait(self, item: bytes) -> None:
        """
        Non-blocking put operation. Attempts to enqueue `item` into the queue immediately.

        :param item: The item to enqueue.
        :raises QueueFull: If the queue is full.
        """

    def get(self, timeout: float | None = None) -> bytes:
        """
        Blocking get operation. Attempts to dequeue an item from the queue.
        If the queue is empty, it blocks until an item becomes available or the timeout expires.

        :param timeout: Maximum time to wait (seconds), or None for indefinite wait.
        :return: The dequeued item as bytes.
        :raises QueueEmpty: If the queue is empty beyond the timeout.
        """

    def get_nowait(self) -> bytes:
        """
        Non-blocking get operation. Attempts to dequeue an item from the queue immediately.

        :return: The dequeued item as bytes.
        :raises QueueEmpty: If the queue is empty.
        """

    @property
    def element_size(self) -> int:
        """
        Returns the size of a single element in bytes.
        """

    @property
    def maxsize(self) -> int:
        """
        Returns the maximum number of elements the queue can hold.
        """

    def full(self) -> bool:
        """
        Returns True if the queue is full.
        """

    def empty(self) -> bool:
        """
        Returns True if the queue is empty.
        """