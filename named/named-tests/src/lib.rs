#![cfg(test)]

use named::named;
use named::NamedEnumValues;
use named::NamedType;

#[test]
fn named_struct() {
    #[named]
    struct ZeroStruct;
    assert_eq!("ZeroStruct", ZeroStruct::type_name());

    #[named]
    struct EmptyStruct {}
    assert_eq!("EmptyStruct", EmptyStruct::type_name());

    #[named]
    struct TupleStruct(#[expect(unused)] i32, #[expect(unused)] String);
    assert_eq!("TupleStruct", TupleStruct::type_name());

    #[named]
    struct GenericStruct<T, U: std::fmt::Display>(T, U);
    assert_eq!("GenericStruct", GenericStruct::<i32, i32>::type_name());
}

#[test]
fn named_enum() {
    #[named]
    enum TestEnum {
        ZeroVariant,
        TupleVariant(#[expect(unused)] i32, #[expect(unused)] String),
        StructVariant {
            #[expect(unused)]
            a: i32,
            #[expect(unused)]
            b: String,
        },
    }
    assert_eq!("ZeroVariant", TestEnum::ZeroVariant.name());
    assert_eq!("TupleVariant", TestEnum::TupleVariant(0, "".into()).name());
    assert_eq!(
        "StructVariant",
        TestEnum::StructVariant { a: 0, b: "".into() }.name()
    );
    assert_eq!("TestEnum", TestEnum::type_name());

    #[named]
    #[expect(unused)]
    enum GenericEnum<T, U: std::fmt::Display> {
        A(T),
        B(U),
    }
    assert_eq!("GenericEnum", GenericEnum::<i32, i32>::type_name());
}

#[test]
#[named]
fn named_fn() {
    assert_eq!("some_named_function", SOME_NAMED_FUNCTION);
    assert_eq!("named_fn", NAMED_FN);
}

#[expect(unused)]
#[named]
pub(crate) fn some_named_function() {}
