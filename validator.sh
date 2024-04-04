#!/bin/bash
solana-test-validator --bpf-program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc artifacts/whirlpool.so \
--bpf-program 7ahQGWysExobjeZ91RTsNqTCN3kWyHGZ43ud2vB7VVoZ artifacts/liquidity_lockbox.so \
--bpf-program DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB target/deploy/fee_collector.so \
--account Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM fork_whirlpool/Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM.json \
--account 7e8LRrfeeSGfS2SSVGJMZQLQKzYhkBp8VKtt34uJMR4t fork_whirlpool/7e8LRrfeeSGfS2SSVGJMZQLQKzYhkBp8VKtt34uJMR4t.json \
--account 5dMKUYJDsjZkAD3wiV3ViQkuq9pSmWQ5eAzcQLtDnUT3 fork_whirlpool/5dMKUYJDsjZkAD3wiV3ViQkuq9pSmWQ5eAzcQLtDnUT3.json \
--account CLA8hU8SkdCZ9cJVLMfZQfcgAsywZ9txBJ6qrRAqthLx fork_whirlpool/CLA8hU8SkdCZ9cJVLMfZQfcgAsywZ9txBJ6qrRAqthLx.json \
--account 6E8pzDK8uwpENc49kp5xo5EGydYjtamPSmUKXxum4ybb fork_whirlpool/6E8pzDK8uwpENc49kp5xo5EGydYjtamPSmUKXxum4ybb.json \
--account Bk3UK77Bb5hfZr6mbjGMBosYWm596U6CE3jDinvmui5L fork_whirlpool/Bk3UK77Bb5hfZr6mbjGMBosYWm596U6CE3jDinvmui5L.json \
--account 3oJAqTKTCdGvLS9zpoBquWvMjwthu9Np67Qp4W8AT843 fork_whirlpool/3oJAqTKTCdGvLS9zpoBquWvMjwthu9Np67Qp4W8AT843.json \
--account J3eMJUQWLmSsG5VnXVFHCGwakpKmzi4jkNvi3vbCZQ3o fork_whirlpool/J3eMJUQWLmSsG5VnXVFHCGwakpKmzi4jkNvi3vbCZQ3o.json \
-r
