Feature: Check

  Scenario: Check a file
    Given a file named "foo.scm" with:
      """
      foo

      """
    When I successfully run `schemat --check foo.scm`
    Then the exit status should be 0

  Scenario: Check a file not formatted
    Given a file named "foo.scm" with:
      """
        foo

      """
    When I run `schemat --check foo.scm`
    Then the exit status should not be 0
    And the stderr should contain "FAIL\tfoo.scm"

  Scenario: Check an invalid file
    Given a file named "foo.scm" with:
      """
      (
      """
    When I run `schemat --check foo.scm`
    Then the exit status should not be 0
    And the stderr should contain "ERROR"
    And the stderr should contain "foo.scm"

  Scenario: Check files
    Given a file named "foo.scm" with:
      """
      foo

      """
    And a file named "bar.scm" with:
      """
      bar

      """
    When I successfully run `schemat --check foo.scm bar.scm`
    Then the stderr should not contain "foo.scm"
    And the stderr should not contain "bar.scm"

  Scenario: Check files with a glob
    Given a file named "foo.scm" with:
      """
      foo

      """
    And a file named "bar.scm" with:
      """
      bar

      """
    When I successfully run `schemat --check *.scm`
    Then the stderr should not contain "foo.scm"
    And the stderr should not contain "bar.scm"

  Scenario: Check files not formatted
    Given a file named "foo.scm" with:
      """
      foo

      """
    And a file named "bar.scm" with:
      """
        bar
      """
    When I run `schemat --check foo.scm bar.scm`
    Then the exit status should not be 0
    And the stderr should not contain "foo.scm"
    And the stderr should contain "FAIL\tbar.scm"

  Scenario: Check files not formatted with a verbose option
    Given a file named "foo.scm" with:
      """
      foo

      """
    And a file named "bar.scm" with:
      """
        bar
      """
    When I run `schemat --check --verbose foo.scm bar.scm`
    Then the exit status should not be 0
    And the stderr should contain "OK\tfoo.scm"
    And the stderr should contain "FAIL\tbar.scm"

  Scenario: Fail to check stdin
    Given a file named "foo.scm" with:
      """
      foo
      """
    When I run `schemat -c` interactively
    And I pipe in the file "foo.scm"
    Then the exit status should not be 0
    And the stderr should contain:
      """
      cannot check stdin
      """

  Scenario: Respect an exclude option
    Given a file named "foo.scm" with:
      """
        foo
      """
    When I successfully run `schemat -ci *.scm *.scm`
    Then the exit status should be 0

  Scenario: Respect .gitignore file
    Given a file named "foo.scm" with:
      """
        foo
      """
    And a file named ".gitignore" with:
      """
      *.scm
      """
    And I successfully run `git init`
    And I successfully run `git config user.name test`
    And I successfully run `git add .`
    And I successfully run `git commit -m commit`
    When I successfully run `schemat -c *.scm`
    Then the exit status should be 0
