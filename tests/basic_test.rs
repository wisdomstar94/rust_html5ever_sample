use html5ever::{parse_document, serialize::{serialize, Serialize, SerializeOpts}, tendril::{StrTendril, TendrilSink}, Attribute, ExpandedName, QualName};
use std::{borrow::Cow, cell::{Cell, RefCell}, collections::{HashSet, VecDeque}, ptr};
use html5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
type Arena<'arena> = &'arena typed_arena::Arena<Node<'arena>>;
type Ref<'arena> = &'arena Node<'arena>;
type Link<'arena> = Cell<Option<Ref<'arena>>>;
// type LinkCell<'arena> = RefCell<Option<Ref<'arena>>>;

struct Sink<'arena> {
  arena: Arena<'arena>,
  document: Ref<'arena>,
  quirks_mode: QuirksMode,
}

impl<'arena> Sink<'arena> {
  fn new_node(&self, data: NodeData<'arena>) -> Ref<'arena> {
      self.arena.alloc(Node::new(data))
  }

  fn append_common<P, A>(&self, child: NodeOrText<Ref<'arena>>, previous: P, append: A)
  where
      P: FnOnce() -> Option<Ref<'arena>>,
      A: FnOnce(Ref<'arena>),
  {
      let new_node = match child {
          NodeOrText::AppendText(text) => {
              // Append to an existing Text node if we have one.
              if let Some(&Node {
                  data: NodeData::Text { ref contents },
                  ..
              }) = previous()
              {
                  contents.borrow_mut().push_tendril(&text);
                  return;
              }
              self.new_node(NodeData::Text {
                  contents: RefCell::new(text),
              })
          },
          NodeOrText::AppendNode(node) => node,
      };

      append(new_node)
  }
}

impl<'arena> TreeSink for Sink<'arena> {
  type Handle = Ref<'arena>;
  type Output = Ref<'arena>;

  fn finish(self) -> Ref<'arena> {
      self.document
  }

  fn parse_error(&mut self, _: Cow<'static, str>) {}

  fn get_document(&mut self) -> Ref<'arena> {
      self.document
  }

  fn set_quirks_mode(&mut self, mode: QuirksMode) {
      self.quirks_mode = mode;
  }

  fn same_node(&self, x: &Ref<'arena>, y: &Ref<'arena>) -> bool {
      ptr::eq::<Node>(*x, *y)
  }

  fn elem_name<'a>(&self, target: &'a Ref<'arena>) -> ExpandedName<'a> {
      match target.data {
          NodeData::Element { ref name, .. } => name.expanded(),
          _ => panic!("not an element!"),
      }
  }

  fn get_template_contents(&mut self, target: &Ref<'arena>) -> Ref<'arena> {
      if let NodeData::Element {
          template_contents: Some(ref contents),
          ..
      } = target.data
      {
          contents
      } else {
          panic!("not a template element!")
      }
  }

  fn is_mathml_annotation_xml_integration_point(&self, target: &Ref<'arena>) -> bool {
      if let NodeData::Element {
          mathml_annotation_xml_integration_point,
          ..
      } = target.data
      {
          mathml_annotation_xml_integration_point
      } else {
          panic!("not an element!")
      }
  }

  fn create_element(
      &mut self,
      name: QualName,
      attrs: Vec<Attribute>,
      flags: ElementFlags,
  ) -> Ref<'arena> {
      self.new_node(NodeData::Element {
          name,
          attrs: RefCell::new(attrs),
          template_contents: if flags.template {
              Some(self.new_node(NodeData::Document))
          } else {
              None
          },
          mathml_annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,
      })
  }

  fn create_comment(&mut self, text: StrTendril) -> Ref<'arena> {
      self.new_node(NodeData::Comment { contents: text })
  }

  fn create_pi(&mut self, target: StrTendril, data: StrTendril) -> Ref<'arena> {
      self.new_node(NodeData::ProcessingInstruction {
          target: target,
          contents: data,
      })
  }

  fn append(&mut self, parent: &Ref<'arena>, child: NodeOrText<Ref<'arena>>) {
      self.append_common(
          child,
          || parent.last_child.get(),
          |new_node| parent.append(new_node),
      )
  }

  fn append_before_sibling(&mut self, sibling: &Ref<'arena>, child: NodeOrText<Ref<'arena>>) {
      self.append_common(
          child,
          || sibling.previous_sibling.get(),
          |new_node| sibling.insert_before(new_node),
      )
  }

  fn append_based_on_parent_node(
      &mut self,
      element: &Ref<'arena>,
      prev_element: &Ref<'arena>,
      child: NodeOrText<Ref<'arena>>,
  ) {
      if element.parent.get().is_some() {
          self.append_before_sibling(element, child)
      } else {
          self.append(prev_element, child)
      }
  }

  fn append_doctype_to_document(
      &mut self,
      name: StrTendril,
      public_id: StrTendril,
      system_id: StrTendril,
  ) {
      self.document.append(self.new_node(NodeData::Doctype {
          name,
          public_id,
          system_id,
      }))
  }

  fn add_attrs_if_missing(&mut self, target: &Ref<'arena>, attrs: Vec<Attribute>) {
      let mut existing = if let NodeData::Element { ref attrs, .. } = target.data {
          attrs.borrow_mut()
      } else {
          panic!("not an element")
      };

      let existing_names = existing
          .iter()
          .map(|e| e.name.clone())
          .collect::<HashSet<_>>();
      existing.extend(
          attrs
              .into_iter()
              .filter(|attr| !existing_names.contains(&attr.name)),
      );
  }

  fn remove_from_parent(&mut self, target: &Ref<'arena>) {
      target.detach()
  }

  fn reparent_children(&mut self, node: &Ref<'arena>, new_parent: &Ref<'arena>) {
      let mut next_child = node.first_child.get();
      while let Some(child) = next_child {
          debug_assert!(ptr::eq::<Node>(child.parent.get().unwrap(), *node));
          next_child = child.next_sibling.get();
          new_parent.append(child)
      }
  }
}

#[derive(Clone)]
struct Node<'arena> {
  parent: Link<'arena>,
  next_sibling: Link<'arena>,
  previous_sibling: Link<'arena>,
  first_child: Link<'arena>,
  last_child: Link<'arena>,
  // children: LinkCell<'arena>,
  data: NodeData<'arena>,
}

impl<'arena> Node<'arena> {
  fn new(data: NodeData<'arena>) -> Self {
      Node {
          parent: Cell::new(None),
          previous_sibling: Cell::new(None),
          next_sibling: Cell::new(None),
          first_child: Cell::new(None),
          last_child: Cell::new(None),
          // children: RefCell::new(None),
          data,
      }
  }

  fn detach(&self) {
      let parent = self.parent.take();
      let previous_sibling = self.previous_sibling.take();
      let next_sibling = self.next_sibling.take();

      if let Some(next_sibling) = next_sibling {
          next_sibling.previous_sibling.set(previous_sibling);
      } else if let Some(parent) = parent {
          parent.last_child.set(previous_sibling);
      }

      if let Some(previous_sibling) = previous_sibling {
          previous_sibling.next_sibling.set(next_sibling);
      } else if let Some(parent) = parent {
          parent.first_child.set(next_sibling);
      }
  }

  fn append(&'arena self, new_child: &'arena Self) {
      new_child.detach();
      new_child.parent.set(Some(self));
      if let Some(last_child) = self.last_child.take() {
          new_child.previous_sibling.set(Some(last_child));
          debug_assert!(last_child.next_sibling.get().is_none());
          last_child.next_sibling.set(Some(new_child));
      } else {
          debug_assert!(self.first_child.get().is_none());
          self.first_child.set(Some(new_child));
      }
      self.last_child.set(Some(new_child));
  }

  fn insert_before(&'arena self, new_sibling: &'arena Self) {
      new_sibling.detach();
      new_sibling.parent.set(self.parent.get());
      new_sibling.next_sibling.set(Some(self));
      if let Some(previous_sibling) = self.previous_sibling.take() {
          new_sibling.previous_sibling.set(Some(previous_sibling));
          debug_assert!(ptr::eq::<Node>(
              previous_sibling.next_sibling.get().unwrap(),
              self
          ));
          previous_sibling.next_sibling.set(Some(new_sibling));
      } else if let Some(parent) = self.parent.get() {
          debug_assert!(ptr::eq::<Node>(parent.first_child.get().unwrap(), self));
          parent.first_child.set(Some(new_sibling));
      }
      self.previous_sibling.set(Some(new_sibling));
  }

  // fn children(&'arena self) -> Vec<Node<'arena>> {
  //   let mut vec: Vec<Node<'arena>> = Vec::new();
  //   let mut current_child_option: Option<&Node<'arena>> = self.first_child.take();
  //   loop {
  //     if let None = current_child_option {
  //       break;
  //     }
  //     let current_child = current_child_option.unwrap();
  //     vec.push(current_child.clone());
  //     current_child_option = current_child.next_sibling.take();
  //   }
  //   vec
  //   // RefCell::new(vec)
  // }
}

fn get_children<'a>(target_node: &'a Node<'a>) -> Vec<Node<'a>> {
  let mut vec: Vec<Node> = Vec::new();
  let mut current_child_option: Option<&Node> = target_node.first_child.take();
  loop {
    if let None = current_child_option {
      break;
    }
    let current_child = current_child_option.unwrap();
    vec.push(current_child.clone());
    current_child_option = current_child.next_sibling.take();
  }
  vec
}

#[derive(Clone)]
enum NodeData<'arena> {
  Document,
  Doctype {
      name: StrTendril,
      public_id: StrTendril,
      system_id: StrTendril,
  },
  Text {
      contents: RefCell<StrTendril>,
  },
  Comment {
      contents: StrTendril,
  },
  Element {
      name: QualName,
      attrs: RefCell<Vec<Attribute>>,
      template_contents: Option<Ref<'arena>>,
      mathml_annotation_xml_integration_point: bool,
  },
  ProcessingInstruction {
      target: StrTendril,
      contents: StrTendril,
  },
}

// type Handle<'a> = &'a Node<'a>;

enum SerializeOp<'a> {
  Open(Node<'a>),
  Close(QualName),
}

// pub struct SerializableHandle<'a>(Handle<'a>);

// impl From<Handle<'_>> for SerializableHandle<'_> {
//   fn from(h: Handle) -> Self {
//     SerializableHandle(h)
//   }
// }

impl Serialize for Node<'_> {
  fn serialize<S>(&self, serializer: &mut S, traversal_scope: html5ever::serialize::TraversalScope) -> std::io::Result<()>
  where
    S: html5ever::serialize::Serializer 
  {
    println!("serialize 호출됨!");
    let mut ops: VecDeque<SerializeOp<'_>> = VecDeque::new();
    match traversal_scope {
      html5ever::serialize::TraversalScope::IncludeNode => {
        // let clone = *self.clone();
        println!("IncludeNode");
        ops.push_back(SerializeOp::Open(self.clone()));
      },
      html5ever::serialize::TraversalScope::ChildrenOnly(name) => {
        println!("ChildrenOnly : {:#?}", name);
        // let children = get_children(self);
        match self.data.clone() {
            NodeData::Document => { println!("Document") },
            NodeData::Doctype { name, public_id, system_id } => { println!("Doctype") },
            NodeData::Text { contents } => { println!("Text") },
            NodeData::Comment { contents } => { println!("Comment") },
            NodeData::Element { name, attrs, template_contents, mathml_annotation_xml_integration_point } => { println!("Element") },
            NodeData::ProcessingInstruction { target, contents } => { println!("ProcessingInstruction") },
        }

        let mut children: Vec<Node> = Vec::new();
        let mut current_child_option: Option<&Node> = self.first_child.take();
        loop {
          if let None = current_child_option {
            println!("없...다?");
            break;
          }
          let current_child = current_child_option.unwrap();
          children.push(current_child.clone());
          current_child_option = current_child.next_sibling.take();
        }

        ops.extend(children.iter().map(|h| SerializeOp::Open(h.clone())))
      },
      // html5ever::serialize::TraversalScope::ChildrenOnly(_) => {
      //   let clone = self.clone();
      //   let children = clone.children();
      //   for item in &clone.children() {
      //     ops.push_back(SerializeOp::Open(item.clone().clone()));
      //   }
      // },
      // html5ever::serialize::TraversalScope::ChildrenOnly(_) => ops.extend(self
      //   .children()
      //   .iter()
      //   .map(|h| {
      //     let clone = h.clone();
      //     SerializeOp::Open(clone.clone())
      //   })),
      _ => {}
    }

    while let Some(op) = ops.pop_front() {
      match op {
        SerializeOp::Open(handle) => match handle.data {
          NodeData::Element {
              ref name,
              ref attrs,
              ..
          } => {
              serializer.start_elem(
                name.clone(),
                attrs.borrow().iter().map(|at| (&at.name, &at.value[..])),
              )?;

              let mut children: Vec<Node> = Vec::new();
              let mut current_child_option: Option<&Node> = self.first_child.take();
              loop {
                if let None = current_child_option {
                  break;
                }
                let current_child = current_child_option.unwrap();
                children.push(current_child.clone());
                current_child_option = current_child.next_sibling.take();
              }

              ops.reserve(1 + children.len());
              ops.push_front(SerializeOp::Close(name.clone()));

              println!("children.len(): {}", children.len());

              for child in children.iter().rev() {
                println!("child...");
                ops.push_front(SerializeOp::Open(child.clone()));
              }
          },

          NodeData::Doctype { ref name, .. } => serializer.write_doctype(&name)?,

          NodeData::Text { ref contents } => {
              serializer.write_text(&contents.borrow())?
          },

          NodeData::Comment { ref contents } => serializer.write_comment(&contents)?,

          NodeData::ProcessingInstruction {
              ref target,
              ref contents,
          } => serializer.write_processing_instruction(target, contents)?,

          NodeData::Document => panic!("Can't serialize Document node itself"),
        },
        SerializeOp::Close(name) => {
            serializer.end_elem(name)?;
        },
      }
    }

    Ok(())
  }
}

#[test]
fn basic_test() {
  let html = r#"
    <!DOCTYPE html>
    <html>
      <head>
        <title>테스트></title>
      </head>
      <body id="[##_id_##]">
        <s3>
          안녕하십니까
        </s3>
        <my-element>
          반갑습니다.
        </my-element>
      <body>
    </html>
  "#;
  let mut html_bytes = html.as_bytes();

  let arena = typed_arena::Arena::new();
  let sink = Sink {
    arena: &arena,
    document: arena.alloc(Node::new(NodeData::Document)),
    quirks_mode: QuirksMode::NoQuirks,
  };
  let dom = parse_document(sink, Default::default())
    .from_utf8()
    .read_from(&mut html_bytes)
    .unwrap()
  ;

  let html_node = dom.last_child.take().unwrap();
  let body_node = html_node.last_child.take().unwrap();
  let body_child_1_node = body_node.first_child.take().unwrap();
  let body_child_2_node = body_child_1_node.next_sibling.take().unwrap();
  
  // body_node.data.
  // let dom_first_child = &dom.first_child.take();
  // if let Some(v) = dom_first_child {
  //   println!(" ------ 있음 ------ ");
  //   match &v.data {
  //       NodeData::Document => { println!("@@@ Document"); },
  //       NodeData::Doctype { name, public_id, system_id } => { println!("@@@ Doctype"); },
  //       NodeData::Text { contents } => { println!("@@@ Text"); },
  //       NodeData::Comment { contents } => { println!("@@@ Comment"); },
  //       NodeData::Element { name, attrs, template_contents, mathml_annotation_xml_integration_point } => { println!("@@@ Element"); },
  //       NodeData::ProcessingInstruction { target, contents } => { println!("@@@ ProcessingInstruction"); },
  //   }
  // } else {
  //   println!(" ------ 없음 ------ ");
  // }

  // match &dom.first_child.take() {
  //   NodeData::Document => {
  //     println!("Document");
  //   },
  //   NodeData::Doctype { name, public_id, system_id } => {
  //     println!("Doctype");
  //     println!("name: {:#?}", name);
  //     println!("public_id: {:#?}", public_id);
  //     println!("system_id: {:#?}", system_id);
  //   },
  //   NodeData::Text { contents } => {
  //     println!("Text");
  //     println!("contents: {:#?}", contents);
  //   },
  //   NodeData::Comment { contents } => {
  //     println!("Comment");
  //     println!("contents: {:#?}", contents);
  //   },
  //   NodeData::Element { name, attrs, template_contents, mathml_annotation_xml_integration_point } => {
  //     println!("Element");
  //     println!("name: {:#?}", name);
  //     println!("attrs: {:#?}", attrs);
  //     // let c = template_contents.unwrap();
  //     // match &c.data {
  //     //   NodeData::Text { contents } => {
  //     //     println!("Text");
  //     //     println!("contents: {:#?}", contents);
  //     //   },
  //     //   NodeData::Element { name, attrs, template_contents, mathml_annotation_xml_integration_point } => {
  //     //     println!("Element");
  //     //     println!("name: {:#?}", name);
  //     //     println!("attrs: {:#?}", attrs);
  //     //   },
  //     //   NodeData::ProcessingInstruction { target, contents } => {

  //     //   },
  //     //   _ => {

  //     //   },
  //     // }
  //   },
  //   NodeData::ProcessingInstruction { target, contents } => {
  //     println!("ProcessingInstruction");
  //     println!("target: {:#?}", target);
  //     println!("contents: {:#?}", contents);
  //   },
  // }

  // let mut children: Vec<Node> = Vec::new();
  // let mut current_child_option: Option<&Node> = body_node.first_child.take();
  // loop {
  //   if let None = current_child_option {
  //     break;
  //   }
  //   let current_child = current_child_option.unwrap();

  //   println!("@@current_child!!!!!!!! ");
  //   match current_child.data.clone() {
  //     NodeData::Document => {
  //       println!("Document");
  //     },
  //     NodeData::Doctype { name, public_id, system_id } => {
  //       println!("Doctype");
  //       println!("name: {:#?}", name);
  //       println!("public_id: {:#?}", public_id);
  //       println!("system_id: {:#?}", system_id);
  //     },
  //     NodeData::Text { contents } => {
  //       println!("Text");
  //       println!("contents: {:#?}", contents);
  //     },
  //     NodeData::Comment { contents } => {
  //       println!("Comment");
  //       println!("contents: {:#?}", contents);
  //     },
  //     NodeData::Element { name, attrs, template_contents, mathml_annotation_xml_integration_point } => {
  //       println!("Element");
  //       println!("name: {:#?}", name);
  //       println!("attrs: {:#?}", attrs);
  //       // let c = template_contents.unwrap();
  //       // match &c.data {
  //       //   NodeData::Text { contents } => {
  //       //     println!("Text");
  //       //     println!("contents: {:#?}", contents);
  //       //   },
  //       //   NodeData::Element { name, attrs, template_contents, mathml_annotation_xml_integration_point } => {
  //       //     println!("Element");
  //       //     println!("name: {:#?}", name);
  //       //     println!("attrs: {:#?}", attrs);
  //       //   },
  //       //   NodeData::ProcessingInstruction { target, contents } => {
  
  //       //   },
  //       //   _ => {
  
  //       //   },
  //       // }
  //     },
  //     NodeData::ProcessingInstruction { target, contents } => {
  //       println!("ProcessingInstruction");
  //       println!("target: {:#?}", target);
  //       println!("contents: {:#?}", contents);
  //     },
  //   }

  //   children.push(current_child.clone());
  //   current_child_option = current_child.next_sibling.take();
  // }

  let mut bytes = vec![];
  serialize(&mut bytes, body_node, SerializeOpts::default()).unwrap();
  let result = String::from_utf8(bytes).unwrap();
  println!("result:  {:#?}", result);
}