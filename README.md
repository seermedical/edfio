# edfio

## Local development setup

1. Install Rust: https://www.rust-lang.org/tools/install
2. Set up `virtualenv`:
  ```bash
  python -m venv env
  source env/bin/activate
  ```
3. Install Python dependencies: `pip install -r requirements.txt`
4. Build & install the package: `maturin develop`
5. `edfio` is now available in Python.

## Usage

```python
import edfio

reader = edfio.SyncEDFReader("test_file.edf")
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

All attributes of `EDFHeader` and `EDFChannel` can be found in `src/edf_reader/model.rs`
