namespace Hestia.Tui.ViewModels;

internal sealed class RingBuffer<T>
{
    private readonly T[] _buf;
    private int _head;
    private int _count;
    private readonly object _lock = new();

    public int Capacity { get; }

    public RingBuffer(int capacity)
    {
        Capacity = capacity;
        _buf = new T[capacity];
    }

    public void Add(T item)
    {
        lock (_lock)
        {
            if (_count < Capacity)
            {
                _buf[(_head + _count) % Capacity] = item;
                _count++;
            }
            else
            {
                _buf[_head] = item;
                _head = (_head + 1) % Capacity;
            }
        }
    }

    public List<T> Snapshot()
    {
        lock (_lock)
        {
            var result = new List<T>(_count);
            for (int i = 0; i < _count; i++)
                result.Add(_buf[(_head + i) % Capacity]);
            return result;
        }
    }

    public int Count { get { lock (_lock) return _count; } }
}
