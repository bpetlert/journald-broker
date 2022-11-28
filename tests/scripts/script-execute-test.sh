#!/usr/bin/env bash

set | grep --silent --no-messages "JNB_MESSAGE"
if [[ "$?" -ne 0 ]]; then
    echo "FAKE-SCRIPT-ERROR: 'JNB_MESSAGE' environment variable does not exist." >&2
    exit 51
fi

set | grep --silent --no-messages "JNB_JSON"
if [[ "$?" -ne 0 ]]; then
    echo "FAKE-SCRIPT-ERROR: 'JNB_JSON' environment variable does not exist." >&2
    exit 52
fi

if [[ "$JNB_SCRIPT_TEST_CASE" == "1" ]]; then
    echo "FAKE-SCRIPT-ERROR: CASE 1 => Simulate script failure..." >&2
    /usr/bin/ls no-such-file
    exit "$?"
fi

if [[ "$JNB_SCRIPT_TEST_CASE" == "2" ]]; then
    echo "FAKE-SCRIPT-ERROR: CASE 2 => Simulate script timeout..." >&2
    sleep 60
    exit 0
fi

if [[ "$JNB_SCRIPT_TEST_CASE" == "3" ]]; then
    echo "FAKE-SCRIPT-ERROR: CASE 3 => Simulate script normal exit" >&2
    sleep 2
    exit 0
fi

exit 99
