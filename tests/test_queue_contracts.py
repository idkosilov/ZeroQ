from typing import Any

import pytest
from hypothesis import given
from hypothesis import strategies as st
from hypothesis.stateful import (
    RuleBasedStateMachine,
    initialize,
    invariant,
    precondition,
    rule,
)

from zeroq import Empty, Full, Queue


@st.composite
def queue_and_items(draw: Any) -> tuple[int, int, list[bytes]]:
    """Return (element_size, capacity, items) for testing the queue."""
    # Generate a valid element size.
    element_size: int = draw(st.integers(min_value=8, max_value=1024))
    # Generate a capacity that is a power of two.
    capacity: int = draw(
        st.integers(min_value=2, max_value=1024).filter(
            lambda x: (x & (x - 1)) == 0
        )
    )
    # Generate a list of items exactly equal to element_size.
    items: list[bytes] = draw(
        st.lists(
            st.binary(min_size=element_size, max_size=element_size),
            max_size=capacity,
            min_size=0,
        )
    )
    return element_size, capacity, items


@given(data=queue_and_items())
def test_fifo_order(data: tuple[int, int, list[bytes]]) -> None:
    """Test that items are dequeued in FIFO order."""
    element_size, capacity, items = data
    queue: Queue = Queue(
        name='test-fifo',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )

    for item in items:
        queue.put_nowait(item)
    dequeued: list[bytes] = []
    while not queue.empty():
        dequeued.append(queue.get_nowait())

    assert dequeued == items


@given(
    element_size=st.integers(min_value=8, max_value=1024),
    capacity=st.integers(min_value=2, max_value=1024).filter(
        lambda x: (x & (x - 1)) == 0
    ),
)
def test_put_nowait_full(element_size: int, capacity: int) -> None:
    """Test that put_nowait raises Full when the queue is full."""
    queue: Queue = Queue(
        name='test-full',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )
    for _ in range(capacity):
        queue.put_nowait(b'\x00' * element_size)

    with pytest.raises(Full):
        queue.put_nowait(b'\x00' * element_size)


@given(
    element_size=st.integers(min_value=8, max_value=1024),
    capacity=st.integers(min_value=2, max_value=1024).filter(
        lambda x: (x & (x - 1)) == 0
    ),
)
def test_get_nowait_empty(element_size: int, capacity: int) -> None:
    """Test that get_nowait raises Empty when the queue is empty."""
    queue: Queue = Queue(
        name='test-empty',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )

    with pytest.raises(Empty):
        queue.get_nowait()


@given(
    element_size=st.integers(min_value=8, max_value=1024),
    capacity=st.integers(min_value=2, max_value=1024).filter(
        lambda x: (x & (x - 1)) == 0
    ),
)
def test_bool_semantics(element_size: int, capacity: int) -> None:
    """Test that __bool__ reflects the queue empty state."""
    queue: Queue = Queue(
        name='test-bool',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )

    assert not queue, 'Queue should be False when empty'
    queue.put_nowait(b'\x00' * element_size)
    assert queue, 'Queue should be True when non-empty'


@given(data=queue_and_items())
def test_len_invariant(data: tuple[int, int, list[bytes]]) -> None:
    """Test that len(queue) is between 0 and capacity."""
    element_size, capacity, items = data
    queue: Queue = Queue(
        name='test-len',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )

    for idx, item in enumerate(items):
        queue.put_nowait(item)
        assert 0 <= len(queue) <= capacity
        assert len(queue) == idx + 1

    for idx in range(len(items)):
        queue.get_nowait()
        assert 0 <= len(queue) <= capacity
        assert len(queue) == len(items) - idx - 1


class QueueStateMachine(RuleBasedStateMachine):
    """State machine to test queue operations and FIFO ordering."""

    element_size: int
    capacity: int
    queue: Queue
    model: list[bytes]

    @initialize(
        element_size=st.integers(min_value=8, max_value=1024),
        capacity=st.integers(min_value=2, max_value=1024).filter(
            lambda x: (x & (x - 1)) == 0
        ),
    )
    def init_queue(self, element_size: int, capacity: int) -> None:
        """Initialize the queue and the model list."""
        self.element_size = element_size
        self.capacity = capacity
        self.queue = Queue(
            name='state-queue',
            element_size=element_size,
            capacity=capacity,
            create=True,
        )
        self.model = []

    @rule(data=st.data())
    @precondition(lambda self: len(self.model) < self.capacity)
    def enqueue(self, data: Any) -> None:
        """Enqueue an item and update the model list."""
        item: bytes = data.draw(
            st.binary(min_size=self.element_size, max_size=self.element_size)
        )
        self.queue.put_nowait(item)
        self.model.append(item)

    @rule()
    @precondition(lambda self: len(self.model) > 0)
    def dequeue(self) -> None:
        """Dequeue an item and compare with the model's first item."""
        result: bytes = self.queue.get_nowait()
        expected: bytes = self.model.pop(0)
        assert result == expected

    @rule(data=st.data())
    @precondition(lambda self: len(self.model) == self.queue.maxsize)
    def enqueue_full(self, data: Any) -> None:
        """Try to enqueue into a full queue and expect a Full exception."""
        item: bytes = data.draw(
            st.binary(min_size=self.element_size, max_size=self.element_size)
        )
        with pytest.raises(Full):
            self.queue.put_nowait(item)

    @rule()
    @precondition(lambda self: len(self.model) == 0)
    def dequeue_empty(self) -> None:
        """Try to dequeue from an empty queue and expect an Empty exception."""
        with pytest.raises(Empty):
            self.queue.get_nowait()

    @invariant()
    def check_length(self) -> None:
        """Invariant: len(queue) equals model length and is within bounds."""
        qlen: int = len(self.queue)
        assert qlen == len(self.model)
        assert 0 <= qlen <= self.capacity

    @invariant()
    def check_capacity_limit(self) -> None:
        """Invariant: Queue length never exceeds its maximum capacity."""
        assert len(self.queue) <= self.capacity
        assert self.queue.full() == (len(self.queue) == self.capacity)

    @invariant()
    def check_empty_behavior(self) -> None:
        """Invariant: empty() is True if and only if len(queue) == 0."""
        assert self.queue.empty() == (len(self.queue) == 0)

    @invariant()
    def check_put_get_consistency(self) -> None:
        """Invariant: Queue length equals put ops minus get ops."""
        assert len(self.queue) == len(self.model)

    @invariant()
    def check_shared_memory_consistency(self) -> None:
        """Invariant: Another queue instance must see the same length."""
        other_queue: Queue = Queue(name='state-queue', create=False)
        assert len(other_queue) == len(self.queue)

    def teardown(self) -> None:
        """Clean up shared memory after the test run."""
        self.queue.close()


TestQueueStateMachine = QueueStateMachine.TestCase
