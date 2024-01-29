use std::{cell::RefCell, ops::DerefMut, rc::Rc};

#[test]
fn rc_ref_cell_test() {
  let vec = Rc::new(RefCell::new(Vec::<u32>::new()));
  Rc::clone(&vec).borrow_mut().deref_mut().push(3333);

  println!("vec: {:#?}", vec);
}