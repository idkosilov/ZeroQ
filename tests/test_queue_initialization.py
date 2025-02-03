import pytest
from hypothesis import given
from hypothesis import strategies as st

from zeroq import Queue


def powers_of_two(min_exp: int, max_exp: int):
    """Returns a strategy generating powers of two within the given range."""
    return st.builds(
        lambda exp: 2**exp, st.integers(min_value=min_exp, max_value=max_exp)
    )


@given(
    element_size=st.integers(min_value=8, max_value=1024 * 1024),
    capacity=powers_of_two(2, 16),
)
def test_queue_create_valid_parameters(
    element_size: int, capacity: int
) -> None:
    """Tests queue creation with valid element size and capacity."""
    queue = Queue(
        name='test-queue-shm',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )

    assert queue.element_size == element_size
    assert queue.maxsize == capacity
    assert queue.empty()
    assert not queue.full()
    assert len(queue) == 0


@pytest.mark.parametrize(
    ('element_size', 'capacity'),
    [
        (None, None),
        (8, None),
        (None, 8),
    ],
)
def test_queue_create_requires_element_size_and_capacity(
    element_size: int | None, capacity: int | None
):
    """Ensures element size and capacity are required when create=True."""
    with pytest.raises(ValueError, match='required when create=true'):
        Queue(
            name='test-queue',
            element_size=element_size,
            capacity=capacity,
            create=True,
        )


@pytest.mark.parametrize(
    ('element_size', 'capacity'), [(-8, 8), (8, -8), (-16, -16)]
)
def test_queue_create_negative_values(element_size: int, capacity: int):
    """Tests that negative element sizes or capacities raise an error."""
    with pytest.raises(
        OverflowError, match="can't convert negative int to unsigned"
    ):
        Queue(
            name='test-queue',
            element_size=element_size,
            capacity=capacity,
            create=True,
        )


@given(
    element_size=st.integers(min_value=8, max_value=64 * 1024 * 1024),
    capacity=st.integers(min_value=3, max_value=128).filter(
        lambda x: x & (x - 1) != 0
    ),
)
def test_queue_create_invalid_capacity(element_size: int, capacity: int):
    """Ensures queue capacity must be a power of two."""
    with pytest.raises(ValueError, match='must be a power of two'):
        Queue(
            name='test-queue',
            element_size=element_size,
            capacity=capacity,
            create=True,
        )


def test_queue_create_from_existing_segment():
    """Tests that a queue cannot be created if a segment exists."""
    queue = Queue('test-queue', 1, 2, create=True)
    queue.put_nowait(b'1')

    with pytest.raises(OSError, match='specific ID already exists'):
        Queue(
            name='test-queue',
            element_size=8,
            capacity=8,
            create=True,
        )

    assert len(queue) == 1
    assert not queue.empty()
    assert not queue.full()
    assert queue.element_size == 1
    assert queue.maxsize == 2
    assert queue.get_nowait() == b'1'


def test_queue_create_1gb_memory() -> None:
    """Tests queue creation with 1GB of shared memory allocation."""
    max_shm_size = 1024 * 1024 * 1024  # 1 GB
    element_size = 1024 * 1024  # 1 MB
    capacity = max_shm_size // element_size

    queue = Queue(
        name='test-max-memory',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )

    assert queue.element_size == element_size
    assert queue.maxsize == capacity
    assert queue.empty()


@given(
    element_size=st.integers(min_value=8, max_value=1024 * 1024),
    capacity=powers_of_two(2, 16),
)
def test_queue_create_from_existing_valid(
    element_size: int, capacity: int
) -> None:
    """Ensures a queue can be accessed by multiple instances."""
    existing_queue = Queue(
        name='test-queue',
        element_size=element_size,
        capacity=capacity,
        create=True,
    )

    queue = Queue(name='test-queue', create=False)

    assert queue.element_size == element_size
    assert queue.maxsize == capacity
    assert queue.empty()
    assert not queue.full()
    assert len(queue) == 0


def test_queue_create_from_existing_invalid() -> None:
    """Tests that accessing a non-existing queue raises an error."""
    with pytest.raises(OSError, match='Failed to open shared memory'):
        Queue(name='test-queue', create=False)
