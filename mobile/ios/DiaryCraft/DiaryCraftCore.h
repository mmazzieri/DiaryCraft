// DiaryCraft's engine (crates/ffi): one call, JSON in and JSON out. See docs/mobile.md.
#pragma once

// Runs one request. The answer is never null and goes back to diarycraft_free once.
char *diarycraft_call(const char *request);

void diarycraft_free(char *answer);
