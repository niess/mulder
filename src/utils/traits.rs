pub trait MinMax {
    type Number;

    fn min(&self) -> Self::Number;
    fn max(&self) -> Self::Number;

    fn mut_min(&mut self) -> &mut Self::Number;
    fn mut_max(&mut self) -> &mut Self::Number;
}

impl MinMax for (f64, f64) {
    type Number = f64;

    #[inline]
    fn min(&self) -> Self::Number {
        self.0
    }

    #[inline]
    fn max(&self) -> Self::Number {
        self.1
    }

    #[inline]
    fn mut_min(&mut self) -> &mut Self::Number {
        &mut self.0
    }

    #[inline]
    fn mut_max(&mut self) -> &mut Self::Number {
        &mut self.1
    }
}
