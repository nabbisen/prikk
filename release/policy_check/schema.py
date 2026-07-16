"""Focused Draft 2020-12 evaluator for keywords used by the release schema."""

from typing import Any
import re

from .common import json_equal, parse_datetime

KEYWORDS = {
    "$schema", "$id", "$ref", "$defs", "title", "type", "additionalProperties", "required",
    "properties", "const", "enum", "pattern", "items", "minItems", "maxItems", "uniqueItems",
    "allOf", "oneOf", "if", "then", "minLength", "minimum", "format",
}


class SchemaValidator:
    """Validate the repository schema without an external package dependency."""

    def __init__(self, schema: dict[str, Any]):
        self.root = schema
        self._check_keywords(schema, "$")

    def _check_keywords(self, schema: dict[str, Any], path: str) -> None:
        unknown = schema.keys() - KEYWORDS
        if unknown:
            raise ValueError(f"{path}: unsupported schema keyword(s): {', '.join(sorted(unknown))}")
        for container in ("properties", "$defs"):
            for name, child in schema.get(container, {}).items():
                self._check_keywords(child, f"{path}.{container}.{name}")
        for container in ("allOf", "oneOf"):
            for index, child in enumerate(schema.get(container, [])):
                self._check_keywords(child, f"{path}.{container}[{index}]")
        for name in ("items", "if", "then"):
            if name in schema:
                self._check_keywords(schema[name], f"{path}.{name}")

    def validate(self, instance: Any) -> list[str]:
        errors: list[str] = []
        self._check(self.root, instance, "$", errors)
        return errors

    def _resolve(self, reference: str) -> dict[str, Any]:
        if not reference.startswith("#/"):
            raise ValueError(f"unsupported schema reference: {reference}")
        value: Any = self.root
        for part in reference[2:].split("/"):
            value = value[part.replace("~1", "/").replace("~0", "~")]
        return value

    def _valid(self, schema: dict[str, Any], value: Any) -> bool:
        errors: list[str] = []
        self._check(schema, value, "$", errors)
        return not errors

    def _check(self, schema: dict[str, Any], value: Any, path: str, errors: list[str]) -> None:
        if "$ref" in schema:
            self._check(self._resolve(schema["$ref"]), value, path, errors)
        if "const" in schema and not json_equal(value, schema["const"]):
            errors.append(f"{path}: does not equal const")
        if "enum" in schema and not any(json_equal(value, item) for item in schema["enum"]):
            errors.append(f"{path}: is not an allowed value")
        if "type" in schema and not self._has_type(value, schema["type"]):
            errors.append(f"{path}: wrong type")
            return
        if "allOf" in schema:
            for child in schema["allOf"]:
                self._check(child, value, path, errors)
        if "oneOf" in schema:
            matches = sum(self._valid(child, value) for child in schema["oneOf"])
            if matches != 1:
                errors.append(f"{path}: must match exactly one oneOf branch")
        condition = schema.get("if")
        if condition is not None and self._valid(condition, value) and "then" in schema:
            self._check(schema["then"], value, path, errors)
        if isinstance(value, dict):
            self._check_object(schema, value, path, errors)
        elif isinstance(value, list):
            self._check_array(schema, value, path, errors)
        elif isinstance(value, str):
            self._check_string(schema, value, path, errors)
        elif isinstance(value, int) and not isinstance(value, bool):
            if value < schema.get("minimum", value):
                errors.append(f"{path}: below minimum")

    @staticmethod
    def _has_type(value: Any, expected: str) -> bool:
        return {
            "object": isinstance(value, dict),
            "array": isinstance(value, list),
            "string": isinstance(value, str),
            "integer": isinstance(value, int) and not isinstance(value, bool),
            "boolean": isinstance(value, bool),
            "null": value is None,
        }.get(expected, False)

    def _check_object(self, schema: dict[str, Any], value: dict[str, Any], path: str, errors: list[str]) -> None:
        properties = schema.get("properties", {})
        for name in schema.get("required", []):
            if name not in value:
                errors.append(f"{path}: missing {name}")
        if schema.get("additionalProperties") is False:
            for name in value.keys() - properties.keys():
                errors.append(f"{path}: unknown field {name}")
        for name, child in properties.items():
            if name in value:
                self._check(child, value[name], f"{path}.{name}", errors)

    def _check_array(self, schema: dict[str, Any], value: list[Any], path: str, errors: list[str]) -> None:
        if len(value) < schema.get("minItems", 0):
            errors.append(f"{path}: too few items")
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            errors.append(f"{path}: too many items")
        if schema.get("uniqueItems"):
            for index, item in enumerate(value):
                if any(json_equal(item, other) for other in value[index + 1 :]):
                    errors.append(f"{path}: duplicate items")
                    break
        if "items" in schema:
            for index, item in enumerate(value):
                self._check(schema["items"], item, f"{path}[{index}]", errors)

    @staticmethod
    def _check_string(schema: dict[str, Any], value: str, path: str, errors: list[str]) -> None:
        if len(value) < schema.get("minLength", 0):
            errors.append(f"{path}: too short")
        if "pattern" in schema and re.fullmatch(schema["pattern"], value) is None:
            errors.append(f"{path}: pattern mismatch")
        if schema.get("format") == "date-time":
            try:
                parse_datetime(value)
            except (TypeError, ValueError):
                errors.append(f"{path}: invalid date-time")
