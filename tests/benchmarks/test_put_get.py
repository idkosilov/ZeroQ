import multiprocessing
from typing import Protocol

import pytest

import zeroq


class IQueue(Protocol):
    """Interface for queue implementations."""

    def put(self, item: bytes) -> None:
        """Puts an item into the queue."""

    def get(self) -> bytes:
        """Gets an item from the queue."""


def put_item_get_item(queue: IQueue, item: bytes) -> None:
    """Alternates putting and getting items from the queue."""
    queue.put(item)
    queue.get()


queue_types = ['multiprocessing', 'zeroq']

item_sizes = [2**p for p in range(1, 25, 4)]


@pytest.mark.parametrize('queue_type', queue_types)
@pytest.mark.parametrize('item_size', item_sizes)
def test_put_item_get_item(
    benchmark,
    queue_type: type[IQueue],
    item_size: int,
) -> None:
    """Benchmarks alternating put/get operations."""
    queue: IQueue
    if queue_type == 'multiprocessing':
        queue = multiprocessing.Queue()
    else:
        queue = zeroq.Queue(
            name='benchmark',
            element_size=item_size,
            capacity=8,
            create=True,
        )

    item = bytes([1] * item_size)

    benchmark.pedantic(
        put_item_get_item,
        args=(queue, item),
        iterations=1000,
    )
