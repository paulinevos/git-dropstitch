Feature: Undo the last change Git made to this branch

  Scenario: Undo a commit amend twice
    Given the user amended the last commit message twice
    When they undo the last change
    Then the latest commit message is "bar"
    When they undo the last change again
    Then the latest commit message is "foo"

  Scenario: Undo a merge after and an amend
    Given the user merged a branch into the current branch
    When they undo the last change
    Then the latest commit message is "foo"
    When they undo the last change again
    Then the latest commit message is "baz"