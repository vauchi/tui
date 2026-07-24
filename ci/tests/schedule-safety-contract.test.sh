#!/bin/sh
# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
#
# SPDX-License-Identifier: GPL-3.0-or-later

set -eu

ci_config="${1:-.gitlab-ci.yml}"
failures=0

job_block() {
    awk -v job="$1" '
        $0 == job ":" { in_job = 1 }
        in_job && $0 != job ":" && /^[^[:space:]#]/ { exit }
        in_job { print }
    ' "$ci_config"
}

for job in auto-tag:version pages github-mirror
do
    block=$(job_block "$job")
    schedule_line=$(printf '%s\n' "$block" |
        grep -n 'CI_PIPELINE_SOURCE == "schedule"' |
        head -1 | cut -d: -f1 || true)
    default_line=$(printf '%s\n' "$block" |
        grep -n 'CI_COMMIT_BRANCH == \$CI_DEFAULT_BRANCH' |
        head -1 | cut -d: -f1 || true)

    if [ -n "$schedule_line" ] &&
        [ -n "$default_line" ] &&
        [ "$schedule_line" -lt "$default_line" ] &&
        printf '%s\n' "$block" |
        sed -n "${schedule_line},$((schedule_line + 1))p" |
        grep -q 'when: never'; then
        echo "PASS: $job excludes schedules before the default branch"
    else
        echo "FAIL: $job must exclude schedules before the default branch" >&2
        failures=$((failures + 1))
    fi
done

[ "$failures" -eq 0 ]
