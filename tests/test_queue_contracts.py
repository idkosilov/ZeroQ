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

from fastqueue import Empty, Full, Queue


def powers_of_two(min_exp: int, max_exp: int):
    """Return a strategy for powers of two in the given range."""
    return st.builds(
        lambda exp: 2**exp, st.integers(min_value=min_exp, max_value=max_exp)
    )


@st.composite
def queue_and_items(draw):
    """Return (element_size, capacity, items) for testing the queue."""
    # Generate a valid element size.
    element_size = draw(st.integers(min_value=8, max_value=1024))
    # Generate a capacity that is a power of two.
    capacity = draw(
        st.integers(min_value=2, max_value=1024).filter(
            lambda x: (x & (x - 1)) == 0
        )
    )
    # Generate a list of items exactly equal to element_size.
    items = draw(
        st.lists(
            st.binary(min_size=element_size, max_size=element_size),
            max_size=capacity,
            min_size=0,
        )
    )
    return element_size, capacity, items


@given(data=queue_and_items())
def test_fifo_order(data):
    """Test that items are dequeued in FIFO order."""
    element_size, capacity, items = data
    queue = Queue(
        name='test-fifo',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )

    for item in items:
        queue.put_nowait(item)
    dequeued = []
    while not queue.empty():
        dequeued.append(queue.get_nowait())

    assert dequeued == items


@given(
    element_size=st.integers(min_value=8, max_value=1024),
    capacity=st.integers(min_value=2, max_value=1024).filter(
        lambda x: (x & (x - 1)) == 0
    ),
)
def test_put_nowait_full(element_size: int, capacity: int):
    """Test that put_nowait raises Full when the queue is full."""
    queue = Queue(
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
def test_get_nowait_empty(element_size: int, capacity: int):
    """Test that get_nowait raises Empty when the queue is empty."""
    queue = Queue(
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
def test_bool_semantics(element_size: int, capacity: int):
    """Test that __bool__ reflects the queue empty state."""
    queue = Queue(
        name='test-bool',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )

    assert not queue, 'Queue should be False when empty'
    queue.put_nowait(b'\x00' * element_size)
    assert queue, 'Queue should be True when non-empty'


@given(data=queue_and_items())
def test_len_invariant(data):
    """Test that len(queue) is between 0 and capacity."""
    element_size, capacity, items = data
    queue = Queue(
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

    @initialize(
        element_size=st.integers(min_value=8, max_value=1024),
        capacity=st.integers(min_value=2, max_value=1024).filter(
            lambda x: (x & (x - 1)) == 0
        ),
    )
    def init_queue(self, element_size, capacity):
        """Initialize the queue and the model list."""
        self.element_size = element_size
        self.capacity = capacity
        self.queue = Queue(
            name='state-queue',
            element_size=element_size,
            capacity=capacity,
            create=True,
        )
        self.model = []  # A list model to mimic the queue

    @rule(data=st.data())
    @precondition(lambda self: len(self.model) < self.capacity)
    def enqueue(self, data):
        """Enqueue an item and update the model list."""
        item = data.draw(
            st.binary(min_size=self.element_size, max_size=self.element_size)
        )
        self.queue.put_nowait(item)
        self.model.append(item)

    @rule()
    @precondition(lambda self: len(self.model) > 0)
    def dequeue(self):
        """Dequeue an item and compare with the model's first item."""
        result = self.queue.get_nowait()
        expected = self.model.pop(0)
        assert result == expected

    @invariant()
    def check_length(self):
        """Invariant: len(queue) equals model length and is within bounds."""
        qlen = len(self.queue)
        assert qlen == len(self.model)
        assert 0 <= qlen <= self.capacity

    @invariant()
    def check_capacity_limit(self):
        """Invariant: Queue length never exceeds its maximum capacity."""
        assert len(self.queue) <= self.capacity
        assert self.queue.full() == (len(self.queue) == self.capacity)

    @invariant()
    def check_empty_behavior(self):
        """Invariant: empty() is True if and only if len(queue) == 0."""
        assert self.queue.empty() == (len(self.queue) == 0)

    @invariant()
    def check_put_get_consistency(self):
        """Invariant: Queue length equals put ops minus get ops."""
        assert len(self.queue) == len(self.model)

    @invariant()
    def check_blocking_behavior(self):
        """Invariant: put on full queue and get on empty queue must fail."""
        if self.queue.full():
            with pytest.raises(Full):
                self.queue.put(b'\x00' * self.element_size, timeout=0.001)
        if self.queue.empty():
            with pytest.raises(Empty):
                self.queue.get(timeout=0.001)

    @invariant()
    def check_data_integrity(self):
        """Invariant: Dequeued item must have been enqueued before."""
        if self.model:
            observed = self.queue.get_nowait()
            assert observed in self.model, 'Dequeued item was never enqueued'
            self.model.remove(observed)  # Remove only after a successful match

    @invariant()
    def check_shared_memory_consistency(self):
        """Invariant: Another queue instance must see the same length."""
        other_queue = Queue(name='state-queue', create=False)
        assert len(other_queue) == len(self.queue)

    def teardown(self):
        """Clean up shared memory after the test run."""
        if hasattr(self, 'queue') and self.queue:
            del self.queue

        self.queue = None


TestQueueStateMachine = QueueStateMachine.TestCase
