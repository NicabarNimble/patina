#!/bin/bash
exec env PATINA_AI_INTERFACE=gemini patina ai session end --json "$@"
