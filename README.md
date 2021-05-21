# edfio

## Local development setup

1. Install Rust: https://www.rust-lang.org/tools/install

### Python

2. Set up `virtualenv`:
  ```bash
  python -m venv env
  source env/bin/activate
  ```
3. Install Python dependencies: `pip install -r requirements.txt`
4. Build & install the package: `maturin develop`
5. `edfio` is now available in Python.

### JS

2. Install JS dependencies: `npm install`
3. Comment out all Python-related code (`src/lib.rs` and all of `src/edf_reader/python_reader.rs`)
4. Build & install the package: `npm run build`
5. `edfio` is now available in Node

## Usage

Below are a couple of quick, language-specific examples.
All attributes of `EDFHeader` and `EDFChannel` can be found in `src/edf_reader/python_reader.rs` and `src/edf_reader/js_reader.rs`.

### Python

```python
import edfio

reader = edfio.PySyncEDFReader("test_file.edf")
header = reader.header

# Print header values
print(header.start_date)
print(header.number_of_blocks)
print(header.block_duration)

channels = header.channels

# Print header channel values
for channel in channels:
  print(channel.label)
  print(channel.number_of_samples_in_data_record)

# Print data from all channels from offset 0ms to 2000ms as 2D matrix
print(reader.read_data_window(0, 2000))
```

### JS

```javascript
const edfio = require('.')

reader = edfio.readEDF("test_file.edf")
header = reader.header

// Print header values
print(header.startDate)
print(header.numberOfBlocks)
print(header.blockDuration)

channels = header.channels

// Print header channel values
channels.forEach((channel) => {
  print(channel.label)
  print(channel.numberOfSamplesInDataRecord)
})

// readDataWindow not currently implemented for JS package
// print(reader.readDataWindow(0, 2000))
```
