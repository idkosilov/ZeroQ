import multiprocessing
from typing import Callable, Protocol

import pytest

import fastqueue


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


queue_factories = [
    pytest.param(lambda _: multiprocessing.Queue(), id='mp-queue'),
    pytest.param(
        lambda element_size: fastqueue.Queue(
            name='fast-queue-benchmarks',
            element_size=element_size,
            capacity=32,
            create=True,
        ),
        id='fast-queue',
    ),
]

item_sizes = [8, 128, 1024, 256 * 1024, 1024 * 1024 * 8, 1024 * 1024 * 32]


@pytest.mark.parametrize('queue_factory', queue_factories)
@pytest.mark.parametrize('item_size', item_sizes)
def test_put_item_get_item(
    benchmark,
    queue_factory: Callable[[int], IQueue],
    item_size: int,
) -> None:
    """Benchmarks alternating put/get operations."""
    queue = queue_factory(item_size)
    item = bytes([1] * item_size)

    benchmark.pedantic(
        put_item_get_item, args=(queue, item), iterations=1000, rounds=10
    )
