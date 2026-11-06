#[cfg(test)]
mod security_tests {
    use crate::reader::Reader;
    use std::collections::{HashMap, HashSet};
    use std::io::ErrorKind;

    use crate::repr::BinRepr;

    #[test]
    fn reads_and_writes_hash_map() {
        let mut original = HashMap::new();
        original.insert(1u32, String::from("hello"));
        original.insert(2u32, String::from("world"));

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        let mut reader = Reader::from_bytes(&bytes);
        let decoded = reader
            .read::<HashMap<u32, String>>()
            .unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn reads_and_writes_hash_set() {
        let original: HashSet<u32> = [10, 20, 30].into_iter().collect();

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        let mut reader = Reader::from_bytes(&bytes);
        let decoded = reader.read::<HashSet<u32>>().unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn rejects_duplicate_hash_map_keys() {
        // Two entries with the same key.
        let bytes = [
            2, 0, 0, 0,
            1, 0, 0, 0, // key = 1
            10, 0, 0, 0, // value = 10
            1, 0, 0, 0, // duplicate key = 1
            20, 0, 0, 0, // value = 20
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<HashMap<u32, u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_duplicate_hash_set_elements() {
        // Two identical elements.
        let bytes = [
            2, 0, 0, 0,
            42, 0, 0, 0,
            42, 0, 0, 0,
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<HashSet<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_truncated_hash_map_key() {
        let bytes = [
            1, 0, 0, 0,
            42, 0,
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<HashMap<u32, u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_truncated_hash_map_value() {
        let bytes = [
            1, 0, 0, 0,
            42, 0, 0, 0,
            10, 0,
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<HashMap<u32, u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_truncated_hash_set_element() {
        let bytes = [
            1, 0, 0, 0,
            42, 0,
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<HashSet<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn enforces_hash_map_collection_limit() {
        let bytes = [
            3, 0, 0, 0,
            1, 0, 0, 0,
            10, 0, 0, 0,
            2, 0, 0, 0,
            20, 0, 0, 0,
            3, 0, 0, 0,
            30, 0, 0, 0,
        ];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            2,
            100,
            100,
            1024,
            64,
        );

        let result = reader.read::<HashMap<u32, u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn enforces_hash_set_allocation_budget() {
        let bytes = [
            2, 0, 0, 0,
            10, 0, 0, 0,
            20, 0, 0, 0,
        ];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            std::mem::size_of::<u32>(),
            64,
        );

        let result = reader.read::<HashSet<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn enforces_hash_map_nesting_depth() {
        // HashMap<u32, Option<u32>>
        let bytes = [
            1, 0, 0, 0,
            1, 0, 0, 0,
            1,
            42, 0, 0, 0,
        ];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            100,
            1,
        );

        let result = reader.read::<HashMap<u32, Option<u32>>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn accepts_empty_hash_map_with_zero_allocation_budget() {
        let bytes = [0, 0, 0, 0];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            0,
            64,
        );

        let map = reader.read::<HashMap<u32, u32>>().unwrap();

        assert!(map.is_empty());
        assert_eq!(reader.allocated(), 0);
    }

    #[test]
    fn accepts_empty_hash_set_with_zero_allocation_budget() {
        let bytes = [0, 0, 0, 0];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            0,
            64,
        );

        let set = reader.read::<HashSet<u32>>().unwrap();

        assert!(set.is_empty());
        assert_eq!(reader.allocated(), 0);
    }

    #[test]
    fn reads_and_writes_tuple() {
        let original = (42u32, true, String::from("hello"), '🦀');

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        let mut reader = Reader::from_bytes(&bytes);
        let decoded = reader
            .read::<(u32, bool, String, char)>()
            .unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn reads_and_writes_single_element_tuple() {
        let original = (42u32,);

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        let mut reader = Reader::from_bytes(&bytes);
        let decoded = reader.read::<(u32,)>().unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn reads_and_writes_nested_tuple() {
        let original = (
            42u32,
            (true, '🦀'),
            (String::from("hello"), 3.14f64),
        );

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        let mut reader = Reader::from_bytes(&bytes);

        let decoded = reader
            .read::<(u32, (bool, char), (String, f64))>()
            .unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn rejects_truncated_tuple() {
        // Tuple: (u32, u32)
        // Second element is incomplete.
        let bytes = [
            42, 0, 0, 0,
            10, 0,
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<(u32, u32)>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_invalid_bool_inside_tuple() {
        // Tuple: (u32, bool)
        let bytes = [
            42, 0, 0, 0,
            2,
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<(u32, bool)>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_invalid_char_inside_tuple() {
        // Tuple: (u32, char)
        // U+D800 is not a valid Unicode scalar value.
        let bytes = [
            42, 0, 0, 0,
            0x00, 0xD8, 0x00, 0x00,
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<(u32, char)>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn enforces_tuple_nesting_depth() {
        // Tuple containing Option<u32> with Some(42).
        let bytes = [
            1,
            42, 0, 0, 0,
        ];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            100,
            1,
        );

        let result = reader.read::<(Option<u32>,)>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn accepts_empty_allocation_budget_for_fixed_tuple() {
        // Tuple: (u32, bool)
        let bytes = [
            42, 0, 0, 0,
            1,
        ];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            0,
            64,
        );

        let decoded = reader.read::<(u32, bool)>().unwrap();

        assert_eq!(decoded, (42, true));
        assert_eq!(reader.allocated(), 0);
    }

    #[test]
    fn rejects_truncated_string_inside_tuple() {
        // Tuple: (u32, String)
        // String declares 5 bytes but only contains 2.
        let bytes = [
            42, 0, 0, 0,
            5, 0, 0, 0,
            b'h', b'i',
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<(u32, String)>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }
    
    #[test]
    fn reads_and_writes_ascii_char() {
        let original = 'A';

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        assert_eq!(bytes, [65, 0, 0, 0]);

        let mut reader = Reader::from_bytes(&bytes);
        let decoded = reader.read::<char>().unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn reads_and_writes_unicode_char() {
        let original = '🦀';

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        assert_eq!(bytes.len(), 4);

        let mut reader = Reader::from_bytes(&bytes);
        let decoded = reader.read::<char>().unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn reads_and_writes_unicode_boundary_values() {
        let chars = [
            '\0',
            '\u{007F}',
            '\u{0080}',
            '\u{07FF}',
            '\u{0800}',
            '\u{FFFF}',
            '\u{10000}',
            '\u{10FFFF}',
        ];

        for original in chars {
            let mut bytes = Vec::new();
            original.write_to(&mut bytes).unwrap();

            let mut reader = Reader::from_bytes(&bytes);
            let decoded = reader.read::<char>().unwrap();

            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn rejects_unicode_surrogate_values() {
        // U+D800 is a UTF-16 surrogate, not a Unicode scalar value.
        let bytes = [0x00, 0xD8, 0x00, 0x00];

        let mut reader = Reader::from_bytes(&bytes);
        let result = reader.read::<char>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_unicode_value_above_maximum() {
        // U+110000 exceeds the Unicode maximum U+10FFFF.
        let bytes = [0x00, 0x00, 0x11, 0x00];

        let mut reader = Reader::from_bytes(&bytes);
        let result = reader.read::<char>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_truncated_char() {
        let bytes = [0x41, 0x00, 0x00];

        let mut reader = Reader::from_bytes(&bytes);
        let result = reader.read::<char>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn reads_array_of_unicode_chars() {
        let original = ['R', 'u', '🦀', 'т', '語'];

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        let mut reader = Reader::from_bytes(&bytes);
        let decoded = reader.read::<[char; 5]>().unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn rejects_invalid_char_inside_vector() {
        // Two chars: 'A', followed by invalid U+D800.
        let bytes = [
            2, 0, 0, 0,
            65, 0, 0, 0,
            0x00, 0xD8, 0x00, 0x00,
        ];

        let mut reader = Reader::from_bytes(&bytes);
        let result = reader.read::<Vec<char>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn reads_and_writes_fixed_array() {
        let original = [10u32, 20, 30];

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        // No length prefix: exactly 12 bytes.
        assert_eq!(bytes.len(), 12);

        let mut reader = Reader::from_bytes(&bytes);
        let decoded = reader.read::<[u32; 3]>().unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn rejects_truncated_array() {
        let bytes = [
            10u8, 0, 0, 0,
            20, 0,
        ];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<[u32; 2]>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_invalid_bool_inside_array() {
        let bytes = [0u8, 1, 2, 1];

        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<[bool; 4]>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn accepts_empty_array_with_zero_allocation_budget() {
        let bytes: [u8; 0] = [];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            0,
            64,
        );

        let values = reader.read::<[u32; 0]>().unwrap();

        assert_eq!(values, []);
        assert_eq!(reader.allocated(), 0);
    }

    #[test]
    fn enforces_array_nesting_depth() {
        // [Option<u32>; 1] containing Some(42).
        let bytes = [1u8, 42, 0, 0, 0];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            100,
            1,
        );

        let result = reader.read::<[Option<u32>; 1]>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn reads_array_of_strings() {
        let original = [
            String::from("hello"),
            String::from("world"),
        ];

        let mut bytes = Vec::new();
        original.write_to(&mut bytes).unwrap();

        let mut reader = Reader::from_bytes(&bytes);
        let decoded = reader.read::<[String; 2]>().unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn rejects_invalid_bool_value() {
        let bytes = [2u8];
        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<bool>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn accepts_valid_bool_values() {
        let false_bytes = [0u8];
        let true_bytes = [1u8];

        let mut reader = Reader::from_bytes(&false_bytes);
        assert!(!reader.read::<bool>().unwrap());

        let mut reader = Reader::from_bytes(&true_bytes);
        assert!(reader.read::<bool>().unwrap());
    }

    #[test]
    fn rejects_truncated_bool() {
        let bytes: [u8; 0] = [];
        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<bool>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_invalid_option_tag() {
        let bytes = [2u8];
        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<Option<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_truncated_option_value() {
        // Some(u32)
        let bytes = [1u8, 42, 0];
        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<Option<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_vector_exceeding_allocation_budget() {
        // Длина вектора: 2 элемента u32.
        let bytes = [2u8, 0, 0, 0, 10, 0, 0, 0, 20, 0, 0, 0];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            4,
            64,
        );

        let result = reader.read::<Vec<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn enforces_maximum_nesting_depth() {
        // Option<Option<u32>>
        let bytes = [1u8, 1, 42, 0, 0, 0];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            100,
            1,
        );

        let result = reader.read::<Option<Option<u32>>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn accepts_empty_vector_with_zero_allocation_budget() {
        let bytes = [0u8, 0, 0, 0];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            0,
            64,
        );

        let values = reader.read::<Vec<u32>>().unwrap();

        assert!(values.is_empty());
        assert_eq!(reader.allocated(), 0);
    }

    #[test]
    fn rejects_vector_exceeding_collection_limit() {
        let bytes = [
            3u8, 0, 0, 0,
            10, 0, 0, 0,
            20, 0, 0, 0,
            30, 0, 0, 0,
        ];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            2,
            100,
            100,
            1024,
            64,
        );

        let result = reader.read::<Vec<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );

        assert_eq!(reader.allocated(), 0);
    }

    #[test]
    fn enforces_shared_allocation_budget_for_strings() {
        let bytes = [
            2u8, 0, 0, 0,
            5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o',
            5, 0, 0, 0, b'w', b'o', b'r', b'l', b'd',
        ];

        let budget = 2 * std::mem::size_of::<String>() + 9;

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            budget,
            64,
        );

        let result = reader.read::<Vec<String>>();

        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );

        assert_eq!(
            reader.allocated(),
            2 * std::mem::size_of::<String>() + 5
        );
    }
}
