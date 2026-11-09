use vestigium_core::Ring;

#[test]
fn ring_drops_the_oldest() {
    let mut ring = Ring::with_capacity(3);
    for n in 1..=5 {
        ring.push(n);
    }

    assert_eq!(ring.len(), 3);
    assert_eq!(ring.iter().copied().collect::<Vec<_>>(), vec![3, 4, 5]);
}

#[test]
fn ring_drops_old_items_on_request() {
    let mut ring = Ring::with_capacity(10);
    for n in 1..=5 {
        ring.push(n);
    }
    ring.drop_while(|&n| n < 3);

    assert_eq!(ring.iter().copied().collect::<Vec<_>>(), vec![3, 4, 5]);
}
