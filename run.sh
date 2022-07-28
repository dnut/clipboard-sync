#!/bin/bash

cargo run \
    && (echo; echo service completed?)
    || (echo; echo service failed)

echo exiting
