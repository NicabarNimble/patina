#!/bin/bash
exec env PATINA_AI_INTERFACE=gemini patina ai session start --json --adapter gemini "$@"
