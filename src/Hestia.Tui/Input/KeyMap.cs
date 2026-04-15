using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Hestia.Tui.Input;

public sealed class KeyMap
{
    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = true };

    private static string KeyMapPath => RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
        ? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Hestia", "keymap.json")
        : Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".hestia", "keymap.json");

    private static readonly Dictionary<InputAction, KeyBinding> DefaultBindings = new()
    {
        [InputAction.Quit]      = new KeyBinding(ConsoleKey.Q),
        [InputAction.Back]      = new KeyBinding(ConsoleKey.Escape),
        [InputAction.Confirm]   = new KeyBinding(ConsoleKey.Enter),
        [InputAction.MoveUp]    = new KeyBinding(ConsoleKey.UpArrow),
        [InputAction.MoveDown]  = new KeyBinding(ConsoleKey.DownArrow),
        [InputAction.MoveLeft]  = new KeyBinding(ConsoleKey.LeftArrow),
        [InputAction.MoveRight] = new KeyBinding(ConsoleKey.RightArrow),
        [InputAction.Delete]    = new KeyBinding(ConsoleKey.D),
    };

    private readonly Dictionary<InputAction, KeyBinding> _bindings = new(DefaultBindings);

    public InputAction? Resolve(ConsoleKeyInfo key)
    {
        foreach (var (action, binding) in _bindings)
            if (binding.Matches(key))
                return action;
        return null;
    }

    public async Task Rebind(InputAction action, KeyBinding binding)
    {
        _bindings[action] = binding;
        await SaveAsync();
    }

    public async Task LoadAsync()
    {
        if (!File.Exists(KeyMapPath))
            return;

        try
        {
            await using var stream = File.OpenRead(KeyMapPath);
            var dto = await JsonSerializer.DeserializeAsync<Dictionary<string, KeyBindingDto>>(stream, JsonOptions);
            if (dto is null) return;

            foreach (var (name, bindingDto) in dto)
            {
                if (Enum.TryParse<InputAction>(name, out var action) &&
                    Enum.TryParse<ConsoleKey>(bindingDto.Key, out var key))
                {
                    var mods = bindingDto.Modifiers is null
                        ? ConsoleModifiers.None
                        : Enum.Parse<ConsoleModifiers>(bindingDto.Modifiers);
                    _bindings[action] = new KeyBinding(key, mods);
                }
            }
        }
        catch (JsonException)
        {
            // Corrupt file — keep defaults
        }
    }

    public async Task SaveAsync()
    {
        var dir = Path.GetDirectoryName(KeyMapPath)!;
        Directory.CreateDirectory(dir);

        var dto = _bindings.ToDictionary(
            kvp => kvp.Key.ToString(),
            kvp => new KeyBindingDto(
                kvp.Value.Key.ToString(),
                kvp.Value.Modifiers == ConsoleModifiers.None ? null : kvp.Value.Modifiers.ToString()));

        await using var stream = File.Create(KeyMapPath);
        await JsonSerializer.SerializeAsync(stream, dto, JsonOptions);
    }

    private sealed record KeyBindingDto(
        [property: JsonPropertyName("key")] string Key,
        [property: JsonPropertyName("modifiers")] string? Modifiers);
}
