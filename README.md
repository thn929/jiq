# jiq - JSON Interactive Query Tool

Interactive command-line tool for querying JSON data in real-time using jq syntax.

## Requirements


- **jq** - JSON processor (required)
  - MacOS: `brew install jq`
  - Linux: See https://jqlang.org/download/

## Installation

### From Source
```sh
git clone https://github.com/bellicose100xp/jiq
cd jiq
cargo build --release
sudo cp target/release/jiq /usr/local/bin/
```

## Usage

```sh
# Read from file
jiq data.json

# Read from stdin (pipe)
cat data.json | jiq
echo '{"name": "Alice", "age": 30}' | jiq
curl https://api.example.com/data | jiq
```

## Interactive Mode

1. **Type jq queries** in the input field (bottom pane)
2. **See results instantly** in the results pane (top pane)
3. **Press Tab** to switch focus between panes
4. **Scroll results** when focused (arrow keys, j/k, Page Up/Down)
5. **Exit with output:**
   - **Enter** → Output filtered JSON results
   - **Shift+Enter** → Output query string only (for scripting)
   - **q** or **Ctrl+C** → Exit without output

## Keybindings

### Global Keys (work anywhere)
| Key | Action |
|-----|--------|
| `Tab` | Switch focus between Input and Results |
| `Enter` | Exit and output filtered JSON results |
| `Shift+Enter` | Exit and output query string only |
| `q` | Quit without output |
| `Ctrl+C`  | Quit without output |

### Input Field (when focused)
| Key | Action |
|-----|--------|
| Type characters | Edit jq query |
| `←/→` | Move cursor |
| `Home/End` | Jump to start/end of line |
| `Backspace/Delete` | Delete characters |

### Results Pane (when focused)
| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Scroll up/down 1 line |
| `J/K` | Scroll up/down 10 lines |
| `Ctrl+u` / `PageUp` | Scroll up half page |
| `Ctrl+d` / `PageDown` | Scroll down half page |
| `g` / `Home` | Jump to top |
| `G` | Jump to bottom |

## Examples

### Example 1: Filter and copy results
```sh
cat users.json | jiq
# Type: .users[] | select(.active == true)
# Press Enter
# Results copied to clipboard or piped elsewhere
```

### Example 2: Extract query for reuse
```sh
cat complex_data.json | jiq
# Experiment with query: .data.items[] | select(.price > 100) | .name
# Press Shift+Enter to get the query string
# Save query: jiq data.json > my_query.txt
```

### Example 3: Pipeline usage
```sh
# Get query interactively, then use in script
QUERY=$(cat data.json | jiq <<< "" | tail -1)  # Shift+Enter to get query
echo $QUERY | xargs -I {} jq {} data.json
```

## Tips

- **Empty query** shows original JSON (identity filter `.`)
- **Invalid queries** display jq error messages in red
- **Results auto-scroll** to top when query changes
- **Help text** at bottom shows available keys for focused pane

## License

Licensed under MIT OR Apache-2.0
