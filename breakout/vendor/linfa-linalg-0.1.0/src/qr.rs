//! QR decomposition of rectangular matrices.
//!
//! Note that the QR decomposition implemented here is "thin", so `Q` has dimensions `(r, c)` and
//! `R` is `(c, c)`, where `r` and `c` are the dimensions of the original matrix.
//!
//! This module also exports functionality for calculating matrix inverse and the least squares
//! problem.

use crate::{
    check_square, householder,
    index::UncheckedIndex,
    reflection::Reflection,
    triangular::{self, IntoTriangular, UPLO},
    LinalgError, Result,
};

use ndarray::{prelude::*, Data, DataMut, OwnedRepr, RawDataClone};

/// QR decomposition for matrix by value
pub trait QRInto {
    type Decomp;

    /// Decomposes the matrix into semi-orthogonal matrix `Q` and upper-triangular matrix `R`, such
    /// that `Q * R` yields the original matrix. Matrix rows must be equal or greater than number
    /// of columns.
    fn qr_into(self) -> Result<Self::Decomp>;
}

impl<A: NdFloat, S: DataMut<Elem = A>> QRInto for ArrayBase<S, Ix2> {
    type Decomp = QRDecomp<A, S>;

    fn qr_into(mut self) -> Result<Self::Decomp> {
        let (rows, cols) = self.dim();
        if self.nrows() < self.ncols() {
            return Err(LinalgError::NotThin { rows, cols });
        }

        let mut diag = Array::zeros(cols);
        for i in 0..cols {
            diag[i] = householder::clear_column(&mut self, i, 0);
        }

        Ok(QRDecomp { qr: self, diag })
    }
}

/// QR decomposition for matrix by reference
pub trait QR {
    type Decomp;

    /// Decomposes the matrix into semi-orthogonal matrix `Q` and upper-triangular matrix `R`, such
    /// that `Q * R` yields the original matrix. Matrix rows must be equal or greater than number
    /// of columns.
    fn qr(&self) -> Result<Self::Decomp>;
}

impl<A: NdFloat, S: Data<Elem = A>> QR for ArrayBase<S, Ix2> {
    type Decomp = QRDecomp<A, OwnedRepr<A>>;

    fn qr(&self) -> Result<Self::Decomp> {
        self.to_owned().qr_into()
    }
}

#[derive(Debug)]
/// Compact representation of a QR decomposition. Can be used to yield the `Q` and `R` matrices or
/// to calculate the inverse or solve a system.
pub struct QRDecomp<A, S: DataMut<Elem = A>> {
    // qr must be a "tall" matrix (rows >= cols)
    qr: ArrayBase<S, Ix2>,
    // diag length must be equal to qr.ncols
    diag: Array1<A>,
}

impl<A: Clone, S: DataMut<Elem = A> + RawDataClone> Clone for QRDecomp<A, S> {
    fn clone(&self) -> Self {
        Self {
            qr: self.qr.clone(),
            diag: self.diag.clone(),
        }
    }
}

impl<A: NdFloat, S: DataMut<Elem = A>> QRDecomp<A, S> {
    /// Generate semi-orthogonal `Q` matrix
    pub fn generate_q(&self) -> Array2<A> {
        householder::assemble_q(&self.qr, 0, |i| self.diag[i])
    }

    /// Consumes `self` to generate the upper-triangular `R` matrix
    pub fn into_r(self) -> ArrayBase<S, Ix2> {
        let ncols = self.qr.ncols();
        let mut r = self.qr.slice_move(s![..ncols, ..ncols]);
        // Should zero out the lower-triangular portion (not the diagonal)
        r.triangular_inplace(UPLO::Upper).unwrap();
        r.diag_mut().assign(&self.diag.mapv_into(A::abs));
        r
    }

    /// Generate both `Q` and `R`
    pub fn into_decomp(self) -> (Array2<A>, ArrayBase<S, Ix2>) {
        let q = self.generate_q();
        (q, self.into_r())
    }

    /// Performs `Q.t * b` in place, without actually producing `Q`.
    ///
    /// `b` must have at least R rows, although the output will only reside in the first C rows of
    /// `b` (R and C are the dimensions of the decomposed matrix).
    fn qt_mul<Si: DataMut<Elem = A>>(&self, b: &mut ArrayBase<Si, Ix2>) {
        let cols = self.qr.ncols();
        for i in 0..cols {
            let axis = self.qr.slice(s![i.., i]);
            let refl = Reflection::new(axis, A::zero());

            let mut rows = b.slice_mut(s![i.., ..]);
            refl.reflect_cols(&mut rows);
            rows *= self.diag[i].signum();
        }
    }

    /// Solves `A * x = b`, where `A` is the original matrix. Used to calculate least squares for
    /// "thin" matrices (rows >= cols).
    pub fn solve_into<Si: DataMut<Elem = A>>(
        &self,
        mut b: ArrayBase<Si, Ix2>,
    ) -> Result<ArrayBase<Si, Ix2>> {
        if self.qr.nrows() != b.nrows() {
            return Err(LinalgError::WrongRows {
                expected: self.qr.nrows(),
                actual: b.nrows(),
            });
        }
        if !self.is_invertible() {
            return Err(LinalgError::NonInvertible);
        }

        // Calculate Q.t * b and extract the result
        self.qt_mul(&mut b);
        let ncols = self.qr.ncols();
        let mut b = b.slice_move(s![..ncols, ..]);

        // Equivalent to solving R * x = Q.t * b
        // This gives the solution to the linear problem
        triangular::solve_triangular_system(
            &self.qr.slice(s![..ncols, ..ncols]),
            &mut b,
            UPLO::Upper,
            |i| unsafe { self.diag.at(i).abs() },
        )?;
        Ok(b)
    }

    /// Solves `A.t * x = b`, where `A` is the original matrix. Used to calculate least squares for
    /// "wide" matrices (rows < cols).
    pub fn solve_tr_into<Si: DataMut<Elem = A>>(
        &self,
        mut b: ArrayBase<Si, Ix2>,
    ) -> Result<Array2<A>> {
        if self.qr.ncols() != b.nrows() {
            return Err(LinalgError::WrongRows {
                expected: self.qr.ncols(),
                actual: b.nrows(),
            });
        }
        if !self.is_invertible() {
            return Err(LinalgError::NonInvertible);
        }

        let ncols = self.qr.ncols();
        // Equivalent to solving R.t * m = b, where m is upper portion of x
        triangular::solve_triangular_system(
            &self.qr.slice(s![..ncols, ..ncols]).t(),
            &mut b,
            UPLO::Lower,
            |i| unsafe { self.diag.at(i).abs() },
        )?;

        // XXX Could implement a non-transpose version of qt_mul to reduce allocations
        Ok(self.generate_q().dot(&b))
    }

    /// Solves `A * x = b`, where `A` is the original matrix.
    pub fn solve<Si: Data<Elem = A>>(&self, b: &ArrayBase<Si, Ix2>) -> Result<Array2<A>> {
        self.solve_into(b.to_owned())
    }

    /// Solves `A.t * x = b`, where `A` is the original matrix.
    pub fn solve_tr<Si: Data<Elem = A>>(&self, b: &ArrayBase<Si, Ix2>) -> Result<Array2<A>> {
        self.solve_tr_into(b.to_owned())
    }

    /// Checks if original matrix is invertible.
    pub fn is_invertible(&self) -> bool {
        // No zeros in the diagonal
        self.diag.iter().all(|f| !f.is_zero())
    }

    /// Produce the inverse of the original matrix, if it's invertible.
    pub fn inverse(&self) -> Result<Array2<A>> {
        check_square(&self.qr)?;
        self.solve_into(Array2::eye(self.diag.len()))
    }
}

/// Use QR decomposition to calculate least squares by value
pub trait LeastSquaresQrInto<B> {
    type Output;

    /// Find solution to `A * x = b` such that `||A * x - b||^2` is minimized
    fn least_squares_into(self, b: B) -> Result<Self::Output>;
}

impl<A: NdFloat, Si: DataMut<Elem = A>, So: DataMut<Elem = A>>
    LeastSquaresQrInto<ArrayBase<So, Ix2>> for ArrayBase<Si, Ix2>
{
    type Output = Array2<A>;

    fn least_squares_into(self, b: ArrayBase<So, Ix2>) -> Result<Self::Output> {
        let out = if self.nrows() >= self.ncols() {
            self.qr_into()?.solve_into(b)?.into_owned()
        } else {
            // If array is fat (rows < cols) then take the QR of the transpose and run the
            // transpose solving algorithm
            self.reversed_axes().qr_into()?.solve_tr_into(b)?
        };
        Ok(out)
    }
}

/// Use QR decomposition to calculate least squares by reference. The `A` matrix is still passed by
/// value.
pub trait LeastSquaresQr<B> {
    type Output;

    /// Find solution to `A * x = b` such that `||A * x - b||^2` is minimized
    fn least_squares(self, b: &B) -> Result<Self::Output>;
}

impl<A: NdFloat, Si: DataMut<Elem = A>, So: Data<Elem = A>> LeastSquaresQr<ArrayBase<So, Ix2>>
    for ArrayBase<Si, Ix2>
{
    type Output = Array2<A>;

    fn least_squares(self, b: &ArrayBase<So, Ix2>) -> Result<Self::Output> {
        self.least_squares_into(b.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn qr() {
        let arr = array![[3.2, 1.3], [4.4, 5.2], [1.3, 6.7]];
        let (q, r) = arr.qr().unwrap().into_decomp();

        assert_abs_diff_eq!(
            q,
            array![
                [0.5720674, -0.4115578],
                [0.7865927, 0.0301901],
                [0.2324024, 0.9108835]
            ],
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(r, array![[5.594, 6.391], [0., 5.725]], epsilon = 1e-3);

        let zeros = Array2::<f64>::zeros((2, 2));
        let (q, r) = zeros.qr().unwrap().into_decomp();
        assert_abs_diff_eq!(q, Array2::eye(2));
        assert_abs_diff_eq!(r, zeros);
    }

    #[test]
    fn solve() {
        let a = array![[1., 9.80], [-7., 3.3]];
        let x = array![[3.2, 1.3, 4.4], [5.2, 1.3, 6.7]];
        let b = a.dot(&x);
        let sol = a.qr_into().unwrap().solve(&b).unwrap();
        assert_abs_diff_eq!(sol, x, epsilon = 1e-5);

        assert_abs_diff_eq!(
            Array2::<f64>::eye(2)
                .qr_into()
                .unwrap()
                .solve(&Array2::zeros((2, 3)))
                .unwrap(),
            Array2::zeros((2, 3))
        );

        // Test with non-square matrix
        let a = array![[3.2, 1.3], [4.4, 5.2], [1.3, 6.7]];
        let x = array![[3.2, 1.3, 4.4], [5.2, 1.3, 6.7]];
        let b = a.dot(&x);
        let sol = a.qr_into().unwrap().solve(&b).unwrap();
        assert_abs_diff_eq!(sol, x, epsilon = 1e-5);
    }

    #[test]
    fn solve_tr() {
        let a = array![[1., 9.80], [-7., 3.3]];
        let x = array![[3.2, 1.3, 4.4], [5.2, 1.3, 6.7]];
        let b = a.dot(&x);
        let sol = a.reversed_axes().qr_into().unwrap().solve_tr(&b).unwrap();
        assert_abs_diff_eq!(sol, x, epsilon = 1e-5);

        assert_abs_diff_eq!(
            Array2::<f64>::eye(2)
                .qr_into()
                .unwrap()
                .solve_tr(&Array2::zeros((2, 3)))
                .unwrap(),
            Array2::zeros((2, 3))
        );

        // Test with non-square matrix
        let a = array![[3.2, 1.3], [4.4, 5.2], [1.3, 6.7]].reversed_axes();
        let x = array![[3.2, 1.3, 4.4], [5.2, 1.3, 6.7]].reversed_axes();
        let b = a.dot(&x);
        let sol = a.t().to_owned().qr_into().unwrap().solve_tr(&b).unwrap();
        // For some reason we get a different solution than x, but the product is still b
        assert_abs_diff_eq!(b, a.dot(&sol), epsilon = 1e-7);
    }

    #[test]
    fn inverse() {
        let a = array![[1., 9.80], [-7., 3.3]];
        assert_abs_diff_eq!(
            a.qr_into().unwrap().inverse().unwrap(),
            array![[0.04589, -0.1363], [0.09735, 0.0139]],
            epsilon = 1e-4
        );

        assert_abs_diff_eq!(
            Array2::<f64>::eye(2).qr_into().unwrap().inverse().unwrap(),
            Array2::eye(2)
        );
    }

    #[test]
    fn non_invertible() {
        let arr = Array2::<f64>::zeros((2, 2));
        assert!(matches!(
            arr.qr().unwrap().inverse().unwrap_err(),
            LinalgError::NonInvertible
        ));
        assert!(matches!(
            arr.least_squares_into(Array2::zeros((2, 2))).unwrap_err(),
            LinalgError::NonInvertible
        ));

        let wide = Array2::<f64>::zeros((2, 3));
        assert!(matches!(
            wide.least_squares_into(Array2::zeros((2, 2))).unwrap_err(),
            LinalgError::NonInvertible
        ));
    }

    #[test]
    fn qt_mul() {
        let a = array![[1., 9.80], [-7., 3.3]];
        let mut b = array![[3.2, 1.3, 4.4], [5.2, 1.3, 6.7]];
        let qr = a.qr_into().unwrap();
        let res = qr.generate_q().t().dot(&b);
        qr.qt_mul(&mut b);
        assert_abs_diff_eq!(b, res, epsilon = 1e-7);

        // Test with non-square matrix
        let arr = array![[3.2, 1.3], [4.4, 5.2], [1.3, 6.7]];
        let qr = arr.qr_into().unwrap();
        let mut b = array![[3.2, 1.3, 4.4], [5.2, 1.3, 6.7]].reversed_axes();
        let res = qr.generate_q().t().dot(&b);
        qr.qt_mul(&mut b);
        assert_abs_diff_eq!(b.slice(s![..2, ..2]), res, epsilon = 1e-7);
    }

    #[test]
    fn corner() {
        let (q, r) = Array2::<f64>::zeros((0, 0))
            .qr_into()
            .unwrap()
            .into_decomp();
        assert!(q.is_empty());
        assert!(r.is_empty());

        assert!(matches!(
            Array2::<f64>::zeros((2, 3)).qr_into().unwrap_err(),
            LinalgError::NotThin { rows: 2, cols: 3 }
        ));
        assert!(matches!(
            Array2::<f64>::zeros((3, 2))
                .qr_into()
                .unwrap()
                .inverse()
                .unwrap_err(),
            LinalgError::NotSquare { rows: 3, cols: 2 }
        ));
    }
}
