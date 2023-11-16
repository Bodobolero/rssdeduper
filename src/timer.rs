use chrono::Local;
use log::info;
use std::{thread, time};

// run F1 every interval_in_seconds seconds (plus execution time of F1) and F2 close to midnight exactly once a day
// if max_iterations is 0, run forever
pub fn periodic_task<F1, F2>(mut f1: F1, mut f2: F2, interval_in_seconds: u64, max_iterations: u64)
where
    F1: FnMut(),
    F2: FnMut(),
{
    let mut last_run = Local::now().date_naive();
    let mut counter = 0;
    loop {
        let today = Local::now().date_naive();

        // Check if we are past midnight and f2 hasn't run today
        if today != last_run {
            info!(
                "{}: Started invoking midnight function, today is {}, last run was {}",
                module_path!(),
                today,
                last_run
            );
            f2();
            last_run = today;
            info!(
                "{}: Finished invoking midnight function, today is {}, last run was {}",
                module_path!(),
                today,
                last_run
            );
        }
        info!("{}:: Starting iteration {}", module_path!(), counter);
        f1();
        info!("{}:: Finished iteration {}", module_path!(), counter);

        counter += 1;
        if max_iterations > 0 && counter >= max_iterations {
            break;
        }
        // Sleep for interval_in_seconds seconds before next iteration
        thread::sleep(time::Duration::from_secs(interval_in_seconds));
    }
}

#[cfg(test)]
mod tests {
    use super::super::utilities::setup_test_logger;
    use super::*;

    #[test]
    fn test_periodic_task() {
        setup_test_logger();
        let mut mycounter = 0;
        let mut midnight_counter = 0;
        periodic_task(
            || {
                mycounter += 1;
            },
            || {
                println!("Function f2 invoked close to midnight, so test expected to fail close to midnight");
                midnight_counter += 1;
            },
            1,
            3,
        );
        assert_eq!(mycounter, 3);
        assert_eq!(midnight_counter, 0);
    }
}
