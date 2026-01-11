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

  Scenario: Format files with a recursive glob
    Given a file named "foo.scm" with:
      """
        foo
      """
    And a file named "bar/baz.scm" with:
      """
        bar
      """
    When I successfully run `schemat **/*.scm`
    Then a file named "foo.scm" should contain exactly:
      """
      foo
      """
    And a file named "bar/baz.scm" should contain exactly:
      """
      bar
      """

  Scenario: Do not format files in a current directory
    Given a file named "foo.scm" with:
      """
        foo
      """
    When I successfully run `schemat .`
    Then a file named "foo.scm" should contain exactly:
      """
        foo
      """

  Scenario: Format files with a verbose option
    Given a file named "foo.scm" with:
      """
        foo
      """
    And a file named "bar.scm" with:
      """
        bar
      """
    When I successfully run `schemat --verbose foo.scm bar.scm`
    Then a file named "foo.scm" should contain exactly:
      """
      foo
      """
    And a file named "bar.scm" should contain exactly:
      """
      bar
      """
    And the stderr should contain "FORMAT\tfoo.scm"
    And the stderr should contain "FORMAT\tbar.scm"

  Scenario: Format valid and invalid files with a verbose option
    Given a file named "foo.scm" with:
      """
      ()
      """
    And a file named "bar.scm" with:
      """
      (
      """
    When I run `schemat --verbose foo.scm bar.scm`
    Then a file named "foo.scm" should contain exactly:
      """
      ()
      """
    And a file named "bar.scm" should contain exactly:
      """
      (
      """
    And the stderr should contain "FORMAT\tfoo.scm"
    And the stderr should contain "ERROR"
    And the stderr should contain "bar.scm"

  Scenario: Respect an exclude option
    Given a file named "foo.scm" with:
      """
        foo
      """
    When I successfully run `schemat -e *.scm *.scm`
    Then a file named "foo.scm" should contain exactly:
      """
        foo
      """

  Scenario: Format a file outside a Git repository
    Given a file named "foo.scm" with:
      """
        foo
      """
    And I successfully run `git init bar`
    And I cd to "bar"
    And I successfully run `git config user.name me`
    And I successfully run `git commit --allow-empty -m commit`
    When I successfully run `schemat ../foo.scm`
    And I cd to ".."
    Then a file named "foo.scm" should contain exactly:
      """
      foo
      """

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
    And I successfully run `git config user.name me`
    And I successfully run `git add .`
    And I successfully run `git commit -m commit`
    When I successfully run `schemat *.scm`
    Then a file named "foo.scm" should contain exactly:
      """
        foo
      """

  Scenario: Do not format files in a current directory in a Git repository
    Given a file named "foo.scm" with:
      """
        foo
      """
    And I successfully run `git init`
    And I successfully run `git config user.name me`
    And I successfully run `git add .`
    And I successfully run `git commit -m commit`
    When I successfully run `schemat .`
    Then a file named "foo.scm" should contain exactly:
      """
        foo
      """
