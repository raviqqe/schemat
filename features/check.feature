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
    And the stderr should contain "foo.scm"

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
