#[macro_use]
extern crate serde_derive;
extern crate aflak_cake as cake;
extern crate fitrs;
extern crate serde;

use std::borrow::Cow;
use std::sync::{Arc, Mutex};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum IOType {
    Integer,
    Float,
    Str,
    Fits,
    Image1d,
    Image2d,
    Image3d,
}

#[derive(Clone, Debug)]
pub enum IOValue {
    Integer(i64),
    Float(f64),
    Str(String),
    Fits(Arc<Mutex<fitrs::Fits>>),
    Image1d(Vec<f64>),
    Image2d(Vec<Vec<f64>>),
    Image3d(Vec<Vec<Vec<f64>>>),
}

/// Return the type of each IOValue
impl cake::TypeContent for IOValue {
    type Type = IOType;
    fn get_type(&self) -> Self::Type {
        match self {
            &IOValue::Integer(_) => IOType::Integer,
            &IOValue::Float(_) => IOType::Float,
            &IOValue::Str(_) => IOType::Str,
            &IOValue::Fits(_) => IOType::Fits,
            &IOValue::Image1d(_) => IOType::Image1d,
            &IOValue::Image2d(_) => IOType::Image3d,
            &IOValue::Image3d(_) => IOType::Image3d,
        }
    }
}

/// Open FITS file
fn open_fits(input: Vec<Cow<IOValue>>) -> Vec<IOValue> {
    if let IOValue::Str(ref path) = *input[0] {
        vec![
            fitrs::Fits::open(path)
                .map(|fits| IOValue::Fits(Arc::new(Mutex::new(fits))))
                .unwrap(),
        ]
    } else {
        panic!("Expected path as input!")
    }
}

fn fits_to_3d_image(input: Vec<Cow<IOValue>>) -> Vec<IOValue> {
    if let IOValue::Fits(ref fits) = *input[0] {
        let image = {
            let mut file = fits.lock().unwrap();
            let primary_hdu = &mut file[0];
            let data = primary_hdu.read_data();
            match data {
                &fitrs::FitsData::FloatingPoint32(ref image) => {
                    let (x_max, y_max, z_max) = (image.shape[0], image.shape[1], image.shape[2]);
                    let mut frames = Vec::with_capacity(z_max);
                    let mut iter = image.data.iter();
                    for _ in 0..z_max {
                        let mut rows = Vec::with_capacity(y_max);
                        for _ in 0..y_max {
                            let mut values = Vec::with_capacity(x_max);
                            for _ in 0..x_max {
                                let val = iter.next().unwrap();
                                values.push(*val as f64);
                            }
                            rows.push(values);
                        }
                        frames.push(rows);
                    }
                    frames
                }
                _ => unimplemented!(),
            }
        };
        vec![IOValue::Image3d(image)]
    } else {
        panic!("Expectect FITS as input")
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;
    use super::{IOValue, open_fits, fits_to_3d_image};
    #[test]
    fn test_open_fits() {
        let path = IOValue::Str("/home/malik/workspace/lab/aflak/data/test.fits".to_owned());
        let ret_fits = open_fits(vec![Cow::Owned(path)]);
        let ret_3d_image = fits_to_3d_image(vec![Cow::Borrowed(&ret_fits[0])]);
    }
}