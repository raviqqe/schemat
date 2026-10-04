#!/bin/sh

set -e

generate_list() {
  echo '('

  if [ $2 -ne 0 ]; then
    children=$(generate_list $1 $(expr $2 - 1))
  fi

  for index in $(seq $1); do
    if [ $2 -eq 0 ]; then
      echo $index
    else
      echo $children
    fi
  done

  echo ')'
}

generate_list 10 $1
