pub mod images {
    use lazy_static::lazy_static;
    use tinybmp::Bmp;

    use crate::consts;
    lazy_static! {
        pub static ref PAPAJ: Bmp<'static, consts::ColorType> = Bmp::from_slice(
            include_bytes!("../assets/images/papaj.bmp")
        ).unwrap();

        pub static ref PAPAJ_SMOL: Bmp<'static, consts::ColorType> = Bmp::from_slice(
            include_bytes!("../assets/images/papaj_smol.bmp")
        ).unwrap();
    }
}

