using Hestia.Tui.ViewModels;

namespace Hestia.Tui.Tests;

public sealed class RingBufferTests
{
    [Fact]
    public void Add_below_capacity_preserves_all_items()
    {
        var buf = new RingBuffer<int>(5);
        buf.Add(1); buf.Add(2); buf.Add(3);

        var snapshot = buf.Snapshot();
        Assert.Equal([1, 2, 3], snapshot);
        Assert.Equal(3, buf.Count);
    }

    [Fact]
    public void Add_at_capacity_keeps_exact_capacity()
    {
        var buf = new RingBuffer<int>(3);
        buf.Add(1); buf.Add(2); buf.Add(3);

        Assert.Equal(3, buf.Count);
        Assert.Equal([1, 2, 3], buf.Snapshot());
    }

    [Fact]
    public void Add_over_capacity_drops_oldest()
    {
        var buf = new RingBuffer<int>(3);
        buf.Add(1); buf.Add(2); buf.Add(3); buf.Add(4); buf.Add(5);

        Assert.Equal(3, buf.Count);
        Assert.Equal([3, 4, 5], buf.Snapshot());
    }

    [Fact]
    public void Snapshot_returns_items_in_insertion_order()
    {
        var buf = new RingBuffer<string>(4);
        buf.Add("a"); buf.Add("b"); buf.Add("c"); buf.Add("d"); buf.Add("e");
        // wraps: drops "a", keeps b c d e
        Assert.Equal(["b", "c", "d", "e"], buf.Snapshot());
    }

    [Fact]
    public void Empty_buffer_returns_empty_snapshot()
    {
        var buf = new RingBuffer<int>(10);
        Assert.Empty(buf.Snapshot());
        Assert.Equal(0, buf.Count);
    }
}
