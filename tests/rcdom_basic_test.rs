use html5ever::{namespace_url, parse_document, serialize::{serialize, SerializeOpts}, tendril::{StrTendril, TendrilSink}, Attribute, LocalName, QualName};
use markup5ever_rcdom::{Node, RcDom, SerializableHandle};
use std::{ops::{Deref, DerefMut}, rc::Rc};
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

// fn get_attrs<'a>(attrs_refcell: &'a RefCell<Vec<Attribute>>) -> Vec<(&'a Attribute, String, String)> {
//   let mut vec: Vec<(&'a Attribute, String, String)> = Vec::new();
//   // let binding = attrs_refcell.borrow();
//   let k: std::cell::Ref<'a, Vec<Attribute>> = attrs_refcell.borrow();
//   let attrs = k.deref();
//   for attr in attrs {
//     let name = attr.name.local.to_string();
//     let value = attr.value.to_string();
//     vec.push((attr, name, value));
//   }
//   vec
// }

fn get_attr_name_and_value(attribute: &Attribute) -> (String, String) {
  let name = attribute.name.local.to_string();
  let value = attribute.value.to_string();
  (name, value)
}

// fn parse_attr<'a>(attrs: &'a mut Vec<Attribute>) -> Vec<(&'a mut Attribute, String, String)> {
//   let mut vec: Vec<(&'a mut Attribute, String, String)> = Vec::new();
//   for attribute in attrs.deref_mut() {
//     let name = attribute.name.local.to_string();
//     let value = attribute.value.to_string();
//     vec.push((attribute, name, value));
//   }
//   vec
// }

// fn parse_attr<'a>(attrs: &'a mut Vec<Attribute>) -> Vec<(&'a mut Attribute, String, String)> {
//   // let k = attrs.deref();
//   // let mut attr_mutable_list = k;

//   let mut vec: Vec<(&'a mut Attribute, String, String)> = Vec::new();
//   // let list = &mut *attrs;
//   for attribute in attrs {
//     let name = attribute.name.local.to_string();
//     let value = attribute.value.to_string();
//     vec.push((attribute, name, value));
//   }
//   vec
// }

// fn parse_attr<'a>(attrs: &'a RefCell<Vec<Attribute>>) -> Vec<(&'a Attribute, String, String)> {
//   let mut vec: Vec<(&'a Attribute, String, String)> = Vec::new();
//   let k: Ref<'_, Vec<Attribute>> = attrs.borrow();
//   let list = k.deref();
//   for attribute in list {
//     let name = attribute.name.local.to_string();
//     let value = attribute.value.to_string();
//     vec.push((attribute, name, value));
//   }
//   vec
// }

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
  if let markup5ever_rcdom::NodeData::Element{ name, attrs, template_contents, mathml_annotation_xml_integration_point } = &body.data {
    add_attr(body_rc, "attr1", "my-value");
    
    let mut binding = attrs.borrow_mut();
    let attr_mut_vec = binding.deref_mut();
    for attribute in attr_mut_vec {
      let (name, value) = get_attr_name_and_value(&attribute);
      if name == "attr1" {
        attribute.value.clear();
        attribute.value.push_tendril(&From::from("수정~"));
      }
    }
  }

  let document: SerializableHandle = dom.document.clone().into();
  let mut bytes = vec![];
  serialize(&mut bytes, &document, SerializeOpts::default()).unwrap();
  let result = String::from_utf8(bytes).unwrap();
  println!("result:  {:#?}", result);
}