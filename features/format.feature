Feature: Format
  Scenario: Format stdin
    Given a file named "foo.scm" with:
    """
    foo
    """
    When I run `schemat` interactively
    And I pipe in the file "foo.scm"
    Then the exit status should be 0
    And the stdout should contain exactly:
    """
    foo
    """

  Scenario: Format a file
    Given a file named "foo.scm" with:
    """
      foo
    """
    When I successfully run `schemat foo.scm`
    Then a file named "foo.scm" should contain exactly:
    """
    foo
    """

  Scenario: Format files
    Given a file named "foo.scm" with:
    """
      foo
    """
    And a file named "bar.scm" with:
    """
      bar
    """
    When I successfully run `schemat foo.scm bar.scm`
    Then a file named "foo.scm" should contain exactly:
    """
    foo
    """
    And a file named "bar.scm" should contain exactly:
    """
    bar
    """

  Scenario: Format files with a glob
    Given a file named "foo.scm" with:
    """
      foo
    """
    And a file named "bar.scm" with:
    """
      bar
    """
    When I successfully run `schemat *.scm`
    Then a file named "foo.scm" should contain exactly:
    """
    foo
    """
    And a file named "bar.scm" should contain exactly:
    """
    bar
    """
