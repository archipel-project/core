/// Utils to get many references to the elements of an array

pub trait ArrayUtils<T> {
    fn create_ref_iter<'a>(
        &'a self,
        iter: impl Iterator<Item = usize>,
    ) -> impl Iterator<Item = &'a T>
    where
        T: 'a;

    ///get an iterator of mutable reference to the elements at the given indexes, panic if the indexes are out of bounds or the same element is borrowed many times
    fn create_mut_iter<'a>(
        &'a mut self,
        iter: impl Iterator<Item = usize>,
    ) -> impl Iterator<Item = &'a mut T>
    where
        T: 'a;
}

impl<T, const N: usize> ArrayUtils<T> for [T; N] {
    fn create_ref_iter<'a>(
        &'a self,
        iter: impl Iterator<Item = usize>,
    ) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        iter.map(move |i| &self[i])
    }

    #[cfg(not(debug_assertions))]
    fn create_mut_iter<'a>(
        &'a mut self,
        iter: impl Iterator<Item = usize>,
    ) -> impl Iterator<Item = &'a mut T>
    where
        T: 'a,
    {
        unsafe { iter.map(move |i| &mut *(&mut self[i] as *mut T)) }
    }

    #[cfg(debug_assertions)]
    fn create_mut_iter<'a>(
        &'a mut self,
        iter: impl Iterator<Item = usize>,
    ) -> impl Iterator<Item = &'a mut T>
    where
        T: 'a,
    {
        let mut borrowed = [false; N];
        unsafe {
            iter.map(move |i| {
                assert_eq!(
                    borrowed[i], false,
                    "the element at index {} is already borrowed",
                    i
                );
                borrowed[i] = true;
                &mut *(&mut self[i] as *mut T)
            })
        }
    }
}
