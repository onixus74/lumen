use super::*;

#[test]
fn with_different_process_sends_message_when_timer_expires() {
    run!(
        |arc_process| {
            (
                Just(arc_process.clone()),
                milliseconds(),
                strategy::term(arc_process.clone()),
            )
        },
        |(arc_process, milliseconds, message)| {
            let time = arc_process.integer(milliseconds).unwrap();

            let destination_arc_process = process::test(&arc_process);
            let destination = registered_name();

            prop_assert_eq!(
                erlang::register_2::native(
                    arc_process.clone(),
                    destination,
                    destination_arc_process.pid_term(),
                ),
                Ok(true.into())
            );

            let result =
                erlang::start_timer_3::native(arc_process.clone(), time, destination, message);

            prop_assert!(
                result.is_ok(),
                "Timer reference not returned.  Got {:?}",
                result
            );

            let timer_reference = result.unwrap();

            prop_assert!(timer_reference.is_boxed_local_reference());

            let timeout_message = timeout_message(timer_reference, message, &arc_process);

            prop_assert!(!has_message(&destination_arc_process, timeout_message));

            timeout_after(milliseconds);

            prop_assert!(has_message(&destination_arc_process, timeout_message));

            Ok(())
        },
    );
}

#[test]
fn with_same_process_sends_message_when_timer_expires() {
    run!(
        |arc_process| {
            (
                Just(arc_process.clone()),
                milliseconds(),
                strategy::term(arc_process),
            )
        },
        |(arc_process, milliseconds, message)| {
            let time = arc_process.integer(milliseconds).unwrap();
            let destination = registered_name();

            prop_assert_eq!(
                erlang::register_2::native(
                    arc_process.clone(),
                    destination,
                    arc_process.pid_term(),
                ),
                Ok(true.into())
            );

            let result =
                erlang::start_timer_3::native(arc_process.clone(), time, destination, message);

            prop_assert!(
                result.is_ok(),
                "Timer reference not returned.  Got {:?}",
                result
            );

            let timer_reference = result.unwrap();

            prop_assert!(timer_reference.is_boxed_local_reference());

            let timeout_message = timeout_message(timer_reference, message, &arc_process);

            prop_assert!(!has_message(&arc_process, timeout_message));

            timeout_after(milliseconds);

            prop_assert!(has_message(&arc_process, timeout_message));

            Ok(())
        },
    );
}
