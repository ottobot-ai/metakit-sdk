"""Tests for JSON canonicalization."""

from constellation_sdk import canonicalize


class TestCanonicalize:
    """Test RFC 8785 canonicalization."""

    def test_sorts_object_keys(self):
        """Object keys should be sorted alphabetically."""
        result = canonicalize({"b": 2, "a": 1})
        assert result == '{"a":1,"b":2}'

    def test_handles_nested_objects(self):
        """Nested objects should have sorted keys."""
        result = canonicalize({"b": {"d": 4, "c": 3}, "a": 1})
        assert result == '{"a":1,"b":{"c":3,"d":4}}'

    def test_handles_arrays(self):
        """Arrays should maintain their order."""
        result = canonicalize({"arr": [3, 1, 2]})
        assert result == '{"arr":[3,1,2]}'

    def test_handles_strings_with_special_chars(self):
        """Strings with special characters should be properly escaped."""
        result = canonicalize({"text": 'hello "world"'})
        assert result == '{"text":"hello \\"world\\""}'

    def test_handles_unicode(self):
        """Unicode characters should be preserved."""
        result = canonicalize({"text": "caf\u00e9"})
        assert result == '{"text":"caf\u00e9"}'

    def test_handles_null(self):
        """Null values should be serialized as 'null'."""
        result = canonicalize({"a": None, "b": 1})
        assert result == '{"a":null,"b":1}'

    def test_handles_booleans(self):
        """Boolean values should be serialized correctly."""
        result = canonicalize({"active": True, "deleted": False})
        assert result == '{"active":true,"deleted":false}'

    def test_handles_numbers(self):
        """Numbers should be serialized correctly."""
        result = canonicalize({"int": 42, "float": 3.14, "neg": -1})
        assert result == '{"float":3.14,"int":42,"neg":-1}'

    def test_handles_empty_object(self):
        """Empty objects should work."""
        result = canonicalize({})
        assert result == "{}"

    def test_handles_empty_array(self):
        """Empty arrays should work."""
        result = canonicalize([])
        assert result == "[]"

    def test_handles_deeply_nested(self):
        """Deeply nested structures should work."""
        result = canonicalize({"level1": {"level2": {"level3": {"value": "deep"}}}})
        assert result == '{"level1":{"level2":{"level3":{"value":"deep"}}}}'

    def test_is_deterministic(self):
        """Same data should always produce same result."""
        data = {"id": "test", "value": 42, "nested": {"a": 1, "b": 2}}
        result1 = canonicalize(data)
        result2 = canonicalize(data)
        assert result1 == result2
