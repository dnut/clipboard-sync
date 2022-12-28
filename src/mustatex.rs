macro_rules! mustatex {
	($($viz:vis $name:ident: $Type:ty = $init:expr;)*) => {
		$(
			#[allow(unused)]
			$viz mod $name {
				use super::*;

				static mut INNER: std::sync::Mutex<$Type> = std::sync::Mutex::new($init);

				pub fn set(x: $Type) {
					unsafe {
						*INNER.lock().unwrap() = x;
					}
				}

				pub fn get() -> impl std::ops::Deref<Target = $Type> {
					unsafe { INNER.lock().unwrap() }
				}

				pub fn get_mut() -> impl std::ops::DerefMut<Target = $Type> {
					unsafe { INNER.lock().unwrap() }
				}
			}
		)*
	};
}
pub(crate) use mustatex;
