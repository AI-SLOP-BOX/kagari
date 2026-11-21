# Kagari Studio UI Translations (`src/ui/locales/`)

Each `<locale>.json` file in this directory is automatically discovered by `build.rs` at compile time.

## How to add a new language

1. Copy `en.json` to `<locale>.json` (for example `ko.json`, `zh-CN.json`, `fr.json`).
2. Update `_meta`:
   - `label`: Display name shown in **Preferences → General → Language**
   - `keywords`: Space-separated search keywords for the Preferences search bar
   - `command`: Label shown in the Command Palette (`⌘K`)
   - `order`: Optional sort order (default `100`)
3. Translate all string values (keys not present in `<locale>.json` automatically fall back to `en.json`).

No Rust or `.ame` files need to be edited when adding a new language.
