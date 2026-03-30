from __future__ import annotations

from enum import Enum
from typing import Any, Generic, TypeVar

from pydantic import BaseModel, Field


class ResponseStatus(str, Enum):
    OK = "ok"
    PARTIAL = "partial"
    ERROR = "error"


class ErrorCode(str, Enum):
    INVALID_INPUT = "INVALID_INPUT"
    PATH_OUTSIDE_WORKSPACE = "PATH_OUTSIDE_WORKSPACE"
    FILE_NOT_FOUND = "FILE_NOT_FOUND"
    SYMBOL_NOT_FOUND = "SYMBOL_NOT_FOUND"
    UNSUPPORTED_FILE = "UNSUPPORTED_FILE"
    BACKEND_SCRIPT_NOT_FOUND = "BACKEND_SCRIPT_NOT_FOUND"
    BACKEND_DEPENDENCY_NOT_FOUND = "BACKEND_DEPENDENCY_NOT_FOUND"
    BACKEND_EXECUTION_FAILED = "BACKEND_EXECUTION_FAILED"
    BACKEND_INVALID_RESPONSE = "BACKEND_INVALID_RESPONSE"
    RESULT_TRUNCATED = "RESULT_TRUNCATED"


class ErrorItem(BaseModel):
    code: ErrorCode | str
    message: str
    retryable: bool = False
    suggestion: str | None = None
    target: str | None = None
    details: dict[str, Any] = Field(default_factory=dict)


class ResponseMeta(BaseModel):
    query: dict[str, Any] = Field(
        default_factory=dict,
        description="Normalized request payload used to execute the tool.",
    )
    resolvedPath: str | None = Field(
        default=None,
        description="Resolved workspace-relative scope path when a path argument was provided.",
    )
    truncated: bool = Field(
        default=False,
        description="Whether the response data was truncated or pruned for safety.",
    )
    counts: dict[str, int | None] = Field(
        default_factory=dict,
        description="Stable count metadata such as returnedCount and totalMatched.",
    )
    detection: dict[str, str | None] = Field(
        default_factory=dict,
        description="Normalized detection metadata such as effective language/framework when meaningful.",
    )


TData = TypeVar("TData")


class ResponseEnvelope(BaseModel, Generic[TData]):
    tool: str
    status: ResponseStatus
    summary: str
    data: TData
    errors: list[ErrorItem] = Field(default_factory=list)
    meta: ResponseMeta = Field(default_factory=ResponseMeta)
