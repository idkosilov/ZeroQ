class Empty(Exception):  # noqa: N818
    """Raised when the queue is empty."""

class Full(Exception):  # noqa: N818
    """Raised when the queue is full."""

class Queue:
    """A shared-memory MPMC queue."""

    def __init__(
        self,
        name: str,
        element_size: int | None = None,
        capacity: int | None = None,
        create: bool = True,
    ) -> None:
        """Creates or attaches to a shared-memory queue.

        :param name: Shared memory segment name.
        :param element_size: Element size in bytes (required if creating).
        :param capacity: Number of slots (power of two, required if creating).
        :param create: Whether to create a new queue (default=True).

        :raises ValueError: If element_size/capacity is missing when creating.
        :raises OSError: If shared memory creation/opening fails.
        """

    def put(
        self, item: bytes | bytearray, timeout: float | None = None
    ) -> None:
        """Blocking enqueue operation.

        Blocks until space is available or the timeout expires.

        :param item: Item to enqueue.
        :param timeout: Max wait time (seconds), None for indefinite.

        :raises FullError: If queue remains full beyond timeout.
        """

    def put_nowait(self, item: bytes | bytearray) -> None:
        """Non-blocking enqueue operation.

        :param item: Item to enqueue.

        :raises FullError: If the queue is full.
        """

    def get(self, timeout: float | None = None) -> bytes:
        """Blocking dequeue operation.

        Blocks until an item is available or timeout expires.

        :param timeout: Max wait time (seconds), None for indefinite.

        :return: The dequeued item as bytes.

        :raises Empty: If queue remains empty beyond timeout.
        """

    def get_nowait(self) -> bytes:
        """Non-blocking dequeue operation.

        :return: The dequeued item as bytes.

        :raises Empty: If the queue is empty.
        """

    @property
    def element_size(self) -> int:
        """Size of a single element in bytes."""

    @property
    def maxsize(self) -> int:
        """Maximum number of elements the queue can hold."""

    def full(self) -> bool:
        """Returns True if the queue is full."""

    def empty(self) -> bool:
        """Returns True if the queue is empty."""

    def __len__(self) -> int:
        """Returns the number of elements in the queue."""

    def __bool__(self) -> bool:
        """Returns True if the queue is not empty."""
