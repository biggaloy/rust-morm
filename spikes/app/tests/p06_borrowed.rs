//! Proof 6: `ProductRef<'r>` decodes with no per-field allocation and
//! serializes straight to JSON. A counting allocator measures it.

use app::ProductRef;
use morm::{Conn, Db};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;

thread_local! {
    // `const` with no destructor, so reading it never allocates.
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.set(ALLOCS.get() + 1);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.set(ALLOCS.get() + 1);
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn allocs() -> u64 {
    ALLOCS.get()
}

#[tokio::test(flavor = "current_thread")]
async fn borrowed_rows_decode_and_serialize_without_allocating() {
    let admin = app::admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS spike_product;
             CREATE TABLE spike_product (id int8, name text, cost int8, supplier text, photo bytea, tags text);
             INSERT INTO spike_product
               SELECT i, 'product ' || i, i * 100, 'supplier ' || i,
                      CASE WHEN i % 2 = 0 THEN '\\x0102'::bytea END,
                      CASE WHEN i % 3 = 0 THEN 'tag' END
               FROM generate_series(1, 1000) i",
        )
        .await
        .unwrap();
    let db = Db::connect(&app::dsn(), 1).unwrap();
    let rows = db
        .query(r#"SELECT "id", "name", "cost", "supplier", "photo", "tags" FROM "spike_product" ORDER BY "id""#, &[])
        .await
        .unwrap();
    assert_eq!(rows.len(), 1000);

    // Decoding borrows text and bytes from each row's buffer.
    let before = allocs();
    let mut sum = 0usize;
    for row in &rows {
        let p = ProductRef::from_row(row).unwrap();
        sum += p.name.len()
            + p.supplier.len()
            + p.photo.map_or(0, <[u8]>::len)
            + p.tags.map_or(0, str::len);
    }
    let decoding = allocs() - before;
    assert!(sum > 0);
    assert_eq!(decoding, 0, "decoding 1000 rows allocated {decoding} times");

    // The borrowed fields point into the row, not into copies.
    let first = ProductRef::from_row(&rows[1]).unwrap();
    assert_eq!(first.name, "product 2");
    assert_eq!(first.photo, Some(&[1u8, 2][..]));

    // Serializing writes straight into the caller's buffer.
    let mut out = Vec::with_capacity(1 << 20);
    let before = allocs();
    for row in &rows {
        let p = ProductRef::from_row(row).unwrap();
        serde_json::to_writer(&mut out, &p).unwrap();
        out.push(b'\n');
    }
    let serializing = allocs() - before;
    assert_eq!(
        serializing, 0,
        "decoding and serializing allocated {serializing} times"
    );
    let line = out.split(|b| *b == b'\n').nth(5).unwrap();
    assert_eq!(
        std::str::from_utf8(line).unwrap(),
        r#"{"id":6,"name":"product 6","cost":600,"supplier":"supplier 6","photo":[1,2],"tags":"tag"}"#
    );

    // For comparison: the owned model allocates for each text and bytes field.
    let before = allocs();
    for row in &rows {
        let p = <app::Product as morm::Model>::from_row(row).unwrap();
        std::hint::black_box(p);
    }
    let owned = allocs() - before;
    assert!(owned >= 2000, "owned decoding allocated only {owned} times");
}
