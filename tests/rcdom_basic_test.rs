use html5ever::{namespace_url, parse_document, serialize::{serialize, SerializeOpts}, tendril::{StrTendril, TendrilSink}, tree_builder::ElementFlags, Attribute, LocalName, QualName};
use markup5ever_rcdom::{Handle, Node, NodeData, RcDom, SerializableHandle};
use std::{borrow::Borrow, cell::RefCell, ops::{Deref, DerefMut}, rc::Rc};
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

fn modify_attr(node: &Rc<Node>, attr_name: &str, attr_value: &str, if_empty_append: bool) {
  let node_clone = node.clone();
  if let markup5ever_rcdom::NodeData::Element{ name: _, attrs, template_contents: _, mathml_annotation_xml_integration_point: _ } = &node_clone.data {
    let mut is_modified = false;
    for attribute in attrs.borrow_mut().iter_mut() {
      let name = &attribute.name.local.to_string();
      // let value = &attribute.value;
      if name == attr_name {
        is_modified = true;
        attribute.value.clear();
        attribute.value.push_tendril(&From::from(attr_value));
        break;
      }
    }

    if !is_modified && if_empty_append {
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
  }
}

fn get_attr_name_and_value(attribute: &Attribute) -> (String, String) {
  let name = attribute.name.local.to_string();
  let value = attribute.value.to_string();
  (name, value)
}

fn convert_node_to_html_string(node: &Rc<Node>) -> String {
  let mut bytes = vec![];
  let node_handle: SerializableHandle = node.clone().into();
  serialize(&mut bytes, &node_handle, SerializeOpts::default()).unwrap();
  let result = String::from_utf8(bytes).unwrap();
  result
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

fn node_select_one(target_node: &Rc<Node>, search_element_name: &str, search_attr_list: &Option<&Vec<(&str, &str)>>) -> Option<Rc<Node>> {
  let mut result: Option<Rc<Node>> = None;
  let vec = node_select(target_node, search_element_name, search_attr_list);
  if let Some(v) = vec.deref().borrow().first() {
    result = Some(v.1.clone());
  }
  result
}

fn node_delete(target_node: &Rc<Node>) {
  RcDom::default().remove_from_parent(target_node);
}

fn node_create(element_name: &str, attr_list: &Option<&Vec<(&str, &str)>>) -> Rc<Node> {
  let qual = QualName::new(
    None,
    namespace_url!(""),
    LocalName::from(element_name),
  );
  let attrs: Vec<Attribute> = if let Some(v) = attr_list {
    v.iter().map(|x| -> Attribute {
      let attr_qual = QualName::new(
        None,
        namespace_url!(""),
        LocalName::from(x.0),
      );  
      let mut tendril = StrTendril::new();
      tendril.push_tendril(&From::from(x.1));
      Attribute { name: attr_qual, value: tendril }
    }).collect()
  } else {
    vec![]
  };
  let flags: ElementFlags = ElementFlags::default();
  RcDom::default().create_element(qual, attrs, flags)
}

fn node_parent(target_node: &Rc<Node>) -> Option<Rc<Node>> {
  let mut result: Option<Rc<Node>> = None;
  let binding = &target_node.parent.take();
  if let Some(b) = binding {
    if let Some(k) = b.upgrade() {
      result = Some(k);
    }
  }
  result
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
  let document = dom.get_document();

  let node = node_select_one(&document, "body", &None);
  let k = node.unwrap();
  // let parent = node_parent(&k).unwrap();
  let result = convert_node_to_html_string(&k);
  println!("@@result {}", result);

  let result = convert_node_to_html_string(&document);
  println!("result: {}", result);
}