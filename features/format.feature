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

  Scenario: Format a file with glob characters in its name
    Given a file named "foo[0].scm" with:
      """
        foo
      """
    And a file named "foo0.scm" with:
      """
        bar
      """
    When I successfully run `schemat foo[0].scm`
    Then a file named "foo[0].scm" should contain exactly:
      """
      foo
      """
    And a file named "foo0.scm" should contain exactly:
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
    And the stderr should contain:
      """
      FORMAT	foo.scm
      """
    And the stderr should contain:
      """
      FORMAT	bar.scm
      """

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
    And the stderr should contain:
      """
      FORMAT	foo.scm
      """
    And the stderr should contain "ERROR"
    And the stderr should contain "bar.scm"

  Scenario: Respect an ignore option
    Given a file named "foo.scm" with:
      """
        foo
      """
    When I successfully run `schemat -i *.scm *.scm`
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
    And I successfully run `git config user.email foo@bar.com`
    And I successfully run `git commit --allow-empty -m commit`
    When I successfully run `schemat ../foo.scm`
    And I cd to ".."
    Then a file named "foo.scm" should contain exactly:
      """
      foo
      """

  Scenario: Format files outside a Git repository with a glob
    Given a file named "foo.scm" with:
      """
        foo
      """
    And I successfully run `git init bar`
    And I cd to "bar"
    And I successfully run `git config user.name me`
    And I successfully run `git config user.email foo@bar.com`
    And I successfully run `git commit --allow-empty -m commit`
    When I successfully run `schemat ../*.scm`
    And I cd to ".."
    Then a file named "foo.scm" should contain exactly:
      """
      foo
      """

  Scenario: Format an untracked file in a Git repository
    Given a file named "foo.scm" with:
      """
        foo
      """
    And I successfully run `git init`
    When I successfully run `schemat foo.scm`
    Then a file named "foo.scm" should contain exactly:
      """
      foo
      """

  Scenario: Format a tracked file in a subdirectory
    Given a file named "bar/foo.scm" with:
      """
        foo
      """
    And I successfully run `git init`
    And I successfully run `git add .`
    And I cd to "bar"
    When I successfully run `schemat foo.scm`
    And I cd to ".."
    Then a file named "bar/foo.scm" should contain exactly:
      """
      foo
      """

  Scenario: Format tracked files in a subdirectory with a glob
    Given a file named "bar/foo.scm" with:
      """
        foo
      """
    And I successfully run `git init`
    And I successfully run `git add .`
    And I cd to "bar"
    When I successfully run `schemat *.scm`
    And I cd to ".."
    Then a file named "bar/foo.scm" should contain exactly:
      """
      foo
      """

  Scenario: Format a tracked file with glob characters in its name
    Given a file named "foo[0].scm" with:
      """
        foo
      """
    And a file named "foo0.scm" with:
      """
        bar
      """
    And I successfully run `git init`
    And I successfully run `git add .`
    When I successfully run `schemat foo[0].scm`
    Then a file named "foo[0].scm" should contain exactly:
      """
      foo
      """
    And a file named "foo0.scm" should contain exactly:
      """
        bar
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
    And I successfully run `git config user.email foo@bar.com`
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
    And I successfully run `git config user.email foo@bar.com`
    And I successfully run `git add .`
    And I successfully run `git commit -m commit`
    When I successfully run `schemat .`
    Then a file named "foo.scm" should contain exactly:
      """
        foo
      """
