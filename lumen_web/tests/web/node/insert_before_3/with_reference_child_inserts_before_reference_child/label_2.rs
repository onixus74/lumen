use std::convert::TryInto;
use std::sync::Arc;

use liblumen_alloc::erts::exception::Alloc;
use liblumen_alloc::erts::process::code::stack::frame::{Frame, Placement};
use liblumen_alloc::erts::process::{code, Process};
use liblumen_alloc::erts::term::prelude::*;
use liblumen_alloc::ModuleFunctionArity;

use super::label_3;

pub fn place_frame_with_arguments(
    process: &Process,
    placement: Placement,
    document: Term,
) -> Result<(), Alloc> {
    process.stack_push(document)?;
    process.place_frame(frame(), placement);

    Ok(())
}

// Private

// ```elixir
// # label 2
// # pushed to stack: (document)
// # returned form call: {:ok, reference_child}
// # full stack: ({:ok, reference_child}, document)
// # returns: {:ok, parent}
// {:ok, parent} = Lumen.Web.Document.create_element(parent_document, "div")
// :ok = Lumen.Web.Node.append_child(document, parent)
// :ok = Lumen.Web.Node.append_child(parent, reference_child)
// {:ok, new_child} = Lumen.Web.Document.create_element(document, "ul");
// {:ok, inserted_child} = Lumen.Web.insert_before(parent, new_child, reference_child)
// ```
fn code(arc_process: &Arc<Process>) -> code::Result {
    arc_process.reduce();

    let ok_reference_child = arc_process.stack_pop().unwrap();
    assert!(
        ok_reference_child.is_boxed_tuple(),
        "ok_reference_child ({:?}) is not a tuple",
        ok_reference_child
    );
    let ok_reference_child_tuple: Boxed<Tuple> = ok_reference_child.try_into().unwrap();
    assert_eq!(ok_reference_child_tuple.len(), 2);
    assert_eq!(ok_reference_child_tuple[0], Atom::str_to_term("ok"));
    let reference_child = ok_reference_child_tuple[1];
    assert!(reference_child.is_boxed_resource_reference());

    let document = arc_process.stack_pop().unwrap();
    assert!(document.is_boxed_resource_reference());

    label_3::place_frame_with_arguments(
        arc_process,
        Placement::Replace,
        document,
        reference_child,
    )?;

    let parent_tag = arc_process.binary_from_str("div")?;
    lumen_web::document::create_element_2::place_frame_with_arguments(
        arc_process,
        Placement::Push,
        document,
        parent_tag,
    )?;

    Process::call_code(arc_process)
}

fn frame() -> Frame {
    let module_function_arity = Arc::new(ModuleFunctionArity {
        module: super::module(),
        function: super::function(),
        arity: 0,
    });

    Frame::new(module_function_arity, code)
}
