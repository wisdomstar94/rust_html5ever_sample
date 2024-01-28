use html5ever::{namespace_url, ns, parse_document, serialize::{serialize, SerializeOpts}, tendril::{StrTendril, TendrilSink}, Attribute, LocalName, QualName};
use markup5ever_rcdom::{Node, RcDom, SerializableHandle};
use std::{cell::RefCell, ops::Deref, rc::Rc};
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

fn get_attrs(attrs_refcell: &RefCell<Vec<Attribute>>) -> Vec<(String, String)> {
  let mut vec: Vec<(String, String)> = Vec::new();
  let binding = attrs_refcell.borrow();
  let attrs = binding.deref();
  for attr in attrs {
    let name = attr.name.local.to_string();
    let value = attr.value.to_string();
    vec.push((name, value));
  }
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
  // for item in document.children.borrow().deref() {
  //   let n = item.as_ref();
  //   match &n.data {
  //       markup5ever_rcdom::NodeData::Document => {
  //         println!("Document")
  //       },
  //       markup5ever_rcdom::NodeData::Doctype { name, public_id, system_id } => {
  //         println!("Doctype")
  //       },
  //       markup5ever_rcdom::NodeData::Text { contents } => {
  //         println!("Text")
  //       },
  //       markup5ever_rcdom::NodeData::Comment { contents } => {
  //         println!("Comment")
  //       },
  //       markup5ever_rcdom::NodeData::Element { name, attrs, template_contents, mathml_annotation_xml_integration_point } => {
  //         println!("Element")
  //       },
  //       markup5ever_rcdom::NodeData::ProcessingInstruction { target, contents } => {
  //         println!("ProcessingInstruction")
  //       },
  //   }
  // }

  let binding = html.children.borrow();
  let body_rc = binding.last().unwrap();
  let body = body_rc.as_ref();
  if let markup5ever_rcdom::NodeData::Element{ name, attrs, template_contents, mathml_annotation_xml_integration_point } = &body.data {
    add_attr(body_rc, "attr1", "my-value");
    let attr_list = get_attrs(attrs);
    println!("attr_list: {:#?}", attr_list);
  }


  let document: SerializableHandle = dom.document.clone().into();
  let mut bytes = vec![];
  serialize(&mut bytes, &document, SerializeOpts::default()).unwrap();
  let result = String::from_utf8(bytes).unwrap();
  println!("result:  {:#?}", result);
}