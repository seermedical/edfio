/*!
 * edf-reader parse metadata of EDF file and can read block of data from this EDF file
 * spec of EDF format : https://www.edfplus.info/specs/edf.html
 *
 */

extern crate chrono;
extern crate futures;

pub mod file_reader;
mod parser;
pub mod python_reader;
pub mod js_reader;

use std::mem::transmute;

fn get_sample(data: &Vec<u8>, index: usize) -> i16 {
    unsafe { transmute::<[u8; 2], i16>([data[2 * index].to_le(), data[2 * index + 1].to_le()]) }
}

#[cfg(test)]
mod tests {

    use super::get_sample;

    #[test]
    fn convert_byte_array_to_u16() {
        /**
        Javascript code to retreive the same results :

        const buffer = new ArrayBuffer(2*2);

        let view = new DataView(buffer);

        view.setInt16(0, 456,true);
        view.setInt16(2, -4564,true);

        console.log(new Uint8Array(buffer));  ==> Uint8Array [ 200, 1, 44, 238 ]
        */

        assert_eq!(456, get_sample(&vec![200, 1], 0));
        assert_eq!(-4564, get_sample(&vec![44, 238], 0));
    }

}
