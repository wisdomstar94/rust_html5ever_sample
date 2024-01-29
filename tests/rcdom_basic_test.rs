use html5ever::{namespace_url, parse_document, serialize::{serialize, SerializeOpts}, tendril::{StrTendril, TendrilSink}, Attribute, LocalName, QualName};
use markup5ever_rcdom::{Handle, Node, NodeData, RcDom, SerializableHandle};
use std::{cell::RefCell, ops::{Deref, DerefMut}, rc::Rc};
use html5ever::interface::tree_builder::TreeSink;

fn add_attr(node: &Rc<Node>, attr_name: &str, attr_value: &str) {
  let qual = QualName::new(
    None,
    namespace_url!(""),
    LocalName::from(attr_name),
  );
  let mut tendril = StrTendril::new();
  tendril.push_tendril(&From::from(attr_value));

  RcDom::default().add_attrs_if_missing(node, vec![
    Attribute {
      name: qual,
      value: tendril,
    }
  ]);
}

fn get_attr_name_and_value(attribute: &Attribute) -> (String, String) {
  let name = attribute.name.local.to_string();
  let value = attribute.value.to_string();
  (name, value)
}

fn walk(depth: usize, handle: &Handle, vec: Rc<RefCell<Vec<(usize, Rc<Node>)>>>, search_element_name: &str, search_attr_list: &Option<&Vec<(&str, &str)>>) {
  let node = handle;
  match node.data {
    NodeData::Document => println!("#Document"),
    NodeData::Doctype {
      ref name,
      ref public_id,
      ref system_id,
    } => println!("<!DOCTYPE {} \"{}\" \"{}\">", name, public_id, system_id),
    NodeData::Text { ref contents } => {
      println!("#text: {}", contents.borrow().escape_default())
    },
    NodeData::Comment { ref contents } => println!("<!-- {} -->", contents.escape_default()),
    NodeData::Element {
      ref name,
      ref attrs,
      ..
    } => {
      let current_element_name = name.local.to_string();
      let mut is_required_search_attr = false;
      let mut is_exist_matched_attr = false;
      if let Some(search_attrs) = search_attr_list {
        is_required_search_attr = true;
        for attr in attrs.borrow().iter() {
          let (attr_name, attr_value) = get_attr_name_and_value(attr);
          for search_attr in *search_attrs {
            if search_attr.0 == attr_name && search_attr.1 == attr_value {
              is_exist_matched_attr = true;
              break;
            }
          }
        }
      }
      if is_required_search_attr {
        if current_element_name == search_element_name && is_exist_matched_attr {
          vec.deref().borrow_mut().push((depth, node.clone()));
        }
      } else {
        if current_element_name == search_element_name {
          vec.deref().borrow_mut().push((depth, node.clone()));
        }
      }
    },
    NodeData::ProcessingInstruction { .. } => unreachable!(),
  }

  for child in node.children.borrow().iter() {
    walk(depth + 1, child, vec.clone(), search_element_name, search_attr_list);
  }
}

fn node_select(target_node: &Rc<Node>, search_element_name: &str, search_attr_list: &Option<&Vec<(&str, &str)>>) -> Rc<RefCell<Vec<(usize, Rc<Node>)>>> {
  let vec: Rc<RefCell<Vec<(usize, Rc<Node>)>>> = Rc::new(RefCell::new(vec![]));
  walk(0, target_node, vec.clone(), search_element_name, search_attr_list);
  vec
}

#[test]
fn rcdom_basic_test() {
  let html = r#"
    <!DOCTYPE html>
    <html>
      <head>
        <title>테스트></title>
      </head>
      <body id="[##_id_##]">
        <s3>
          테스트 !!!
        </s3>
        <my-element>
          안녕하세요~ ^^
        </my-element>
      <body>
    </html>
  "#;
  let mut dom = parse_document(RcDom::default(), Default::default())
    .from_utf8()
    .read_from(&mut html.as_bytes())
    .unwrap()
  ;

  // let document = dom.document.borrow();

  let document_rc = dom.get_document();
  let document = document_rc.as_ref();
  let binding = document.children.borrow();
  let html = binding.deref().last().unwrap().as_ref();

  let binding = html.children.borrow();
  let body_rc = binding.last().unwrap();
  let body = body_rc.as_ref();
  if let markup5ever_rcdom::NodeData::Element{ name, attrs, template_contents: _, mathml_annotation_xml_integration_point: _ } = &body.data {
    println!("name: {}", name.local.to_string());
    add_attr(body_rc, "attr1", "my-value");

    let mut binding = attrs.borrow_mut();
    let attr_mut_vec = binding.deref_mut();
    for attribute in attr_mut_vec {
      let (name, _) = get_attr_name_and_value(&attribute);
      if name == "attr1" {
        attribute.value.clear();
        attribute.value.push_tendril(&From::from("수정~"));
      }
    }
  }

  let k = node_select(body_rc, "my-element", &None);
  let vec = k.deref().borrow();
  for item in vec.iter() {
    println!("item: {:#?}", item);
  }

  let document: SerializableHandle = dom.document.clone().into();
  let mut bytes = vec![];
  serialize(&mut bytes, &document, SerializeOpts::default()).unwrap();
  let result = String::from_utf8(bytes).unwrap();
  println!("result:  {:#?}", result);
}