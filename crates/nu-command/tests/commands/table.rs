use nu_test_support::nu;

#[test]
fn table_0() {
    let actual = nu!(r#"[[a b, c]; [1 2 3] [4 5 [1 2 3]]] | table"#);
    assert_eq!(
        actual.out,
        "╭───┬───┬───┬────────────────╮\
         │ # │ a │ b │       c        │\
         ├───┼───┼───┼────────────────┤\
         │ 0 │ 1 │ 2 │              3 │\
         │ 1 │ 4 │ 5 │ [list 3 items] │\
         ╰───┴───┴───┴────────────────╯"
    );
}

#[test]
fn table_collapse_0() {
    let actual = nu!(r#"[[a b, c]; [1 2 3] [4 5 [1 2 3]]] | table --collapse"#);
    assert_eq!(
        actual.out,
        "\u{1b}[37m╭───\u{1b}[39m\u{1b}[37m┬───\u{1b}[39m\u{1b}[37m┬───╮\u{1b}[39m\u{1b}[37m│\u{1b}[39m a \u{1b}[37m│\u{1b}[39m b \u{1b}[37m│\u{1b}[39m c \u{1b}[37m│\u{1b}[39m\u{1b}[37m ───\u{1b}[39m\u{1b}[37m ───\u{1b}[39m\u{1b}[37m ─── \u{1b}[39m\u{1b}[37m│\u{1b}[39m 1 \u{1b}[37m│\u{1b}[39m 2 \u{1b}[37m│\u{1b}[39m 3 \u{1b}[37m│\u{1b}[39m\u{1b}[37m ───\u{1b}[39m\u{1b}[37m ───\u{1b}[39m\u{1b}[37m ─── \u{1b}[39m\u{1b}[37m│\u{1b}[39m 4 \u{1b}[37m│\u{1b}[39m 5 \u{1b}[37m│\u{1b}[39m 1 \u{1b}[37m│\u{1b}[39m\u{1b}[37m│\u{1b}[39m   \u{1b}[37m│\u{1b}[39m   \u{1b}[37m ─── \u{1b}[39m\u{1b}[37m│\u{1b}[39m   \u{1b}[37m│\u{1b}[39m   \u{1b}[37m│\u{1b}[39m 2 \u{1b}[37m│\u{1b}[39m\u{1b}[37m│\u{1b}[39m   \u{1b}[37m│\u{1b}[39m   \u{1b}[37m ─── \u{1b}[39m\u{1b}[37m│\u{1b}[39m   \u{1b}[37m│\u{1b}[39m   \u{1b}[37m│\u{1b}[39m 3 \u{1b}[37m│\u{1b}[39m\u{1b}[37m╰───\u{1b}[39m\u{1b}[37m┴───\u{1b}[39m\u{1b}[37m┴───╯\u{1b}[39m"
    );
}

#[test]
fn table_expand_0() {
    let actual = nu!(r#"[[a b, c]; [1 2 3] [4 5 [1 2 3]]] | table --expand"#);
    assert_eq!(
        actual.out,
        "╭───┬───┬───┬───────────╮\
         │ # │ a │ b │     c     │\
         ├───┼───┼───┼───────────┤\
         │ 0 │ 1 │ 2 │         3 │\
         │ 1 │ 4 │ 5 │ ╭───┬───╮ │\
         │   │   │   │ │ 0 │ 1 │ │\
         │   │   │   │ │ 1 │ 2 │ │\
         │   │   │   │ │ 2 │ 3 │ │\
         │   │   │   │ ╰───┴───╯ │\
         ╰───┴───┴───┴───────────╯"
    );
}

#[test]
fn table_expand_deep_0() {
    let actual = nu!(r#"[[a b, c]; [1 2 3] [4 5 [1 2 [1 2 3]]]] | table --expand --expand-deep=2"#);
    assert_eq!(
        actual.out,
        "╭───┬───┬───┬────────────────────────╮\
         │ # │ a │ b │           c            │\
         ├───┼───┼───┼────────────────────────┤\
         │ 0 │ 1 │ 2 │                      3 │\
         │ 1 │ 4 │ 5 │ ╭───┬────────────────╮ │\
         │   │   │   │ │ 0 │              1 │ │\
         │   │   │   │ │ 1 │              2 │ │\
         │   │   │   │ │ 2 │ [list 3 items] │ │\
         │   │   │   │ ╰───┴────────────────╯ │\
         ╰───┴───┴───┴────────────────────────╯"
    );
}

#[test]
fn table_expand_deep_1() {
    let actual = nu!(r#"[[a b, c]; [1 2 3] [4 5 [1 2 [1 2 3]]]] | table --expand --expand-deep=0"#);
    assert_eq!(
        actual.out,
        "╭───┬───┬───┬────────────────╮\
         │ # │ a │ b │       c        │\
         ├───┼───┼───┼────────────────┤\
         │ 0 │ 1 │ 2 │              3 │\
         │ 1 │ 4 │ 5 │ [list 3 items] │\
         ╰───┴───┴───┴────────────────╯"
    );
}

#[test]
fn table_expand_flatten_0() {
    let actual = nu!(r#"[[a b, c]; [1 2 3] [4 5 [1 2 [1 1 1]]]] | table --expand --flatten "#);
    assert_eq!(
        actual.out,
        "╭───┬───┬───┬───────────────╮\
         │ # │ a │ b │       c       │\
         ├───┼───┼───┼───────────────┤\
         │ 0 │ 1 │ 2 │             3 │\
         │ 1 │ 4 │ 5 │ ╭───┬───────╮ │\
         │   │   │   │ │ 0 │     1 │ │\
         │   │   │   │ │ 1 │     2 │ │\
         │   │   │   │ │ 2 │ 1 1 1 │ │\
         │   │   │   │ ╰───┴───────╯ │\
         ╰───┴───┴───┴───────────────╯"
    );
}

#[test]
fn table_expand_flatten_1() {
    let actual = nu!(
        r#"[[a b, c]; [1 2 3] [4 5 [1 2 [1 1 1]]]] | table --expand --flatten --flatten-separator=,"#
    );
    assert_eq!(
        actual.out,
        "╭───┬───┬───┬───────────────╮\
         │ # │ a │ b │       c       │\
         ├───┼───┼───┼───────────────┤\
         │ 0 │ 1 │ 2 │             3 │\
         │ 1 │ 4 │ 5 │ ╭───┬───────╮ │\
         │   │   │   │ │ 0 │     1 │ │\
         │   │   │   │ │ 1 │     2 │ │\
         │   │   │   │ │ 2 │ 1,1,1 │ │\
         │   │   │   │ ╰───┴───────╯ │\
         ╰───┴───┴───┴───────────────╯"
    );
}

#[test]
fn table_expand_flatten_and_deep_1() {
    let actual = nu!(
        r#"[[a b, c]; [1 2 3] [4 5 [1 2 [1 [1 1 1] 1]]]] | table --expand --expand-deep=2 --flatten --flatten-separator=,"#
    );

    assert_eq!(
        actual.out,
        "╭───┬───┬───┬────────────────────────────────╮\
         │ # │ a │ b │               c                │\
         ├───┼───┼───┼────────────────────────────────┤\
         │ 0 │ 1 │ 2 │                              3 │\
         │ 1 │ 4 │ 5 │ ╭───┬────────────────────────╮ │\
         │   │   │   │ │ 0 │                      1 │ │\
         │   │   │   │ │ 1 │                      2 │ │\
         │   │   │   │ │ 2 │ ╭───┬────────────────╮ │ │\
         │   │   │   │ │   │ │ 0 │              1 │ │ │\
         │   │   │   │ │   │ │ 1 │ [list 3 items] │ │ │\
         │   │   │   │ │   │ │ 2 │              1 │ │ │\
         │   │   │   │ │   │ ╰───┴────────────────╯ │ │\
         │   │   │   │ ╰───┴────────────────────────╯ │\
         ╰───┴───┴───┴────────────────────────────────╯"
    );
}
