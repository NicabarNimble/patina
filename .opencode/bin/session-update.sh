#!/bin/bash
exec env PATINA_AI_INTERFACE=opencode patina ai session update --json "$@"
