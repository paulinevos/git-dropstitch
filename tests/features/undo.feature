Feature: Undo the last change Git made to this branch

  Scenario: Undo a commit amend
    Given the user amended the last commit message twice
    When they undo the last change
    Then the latest commit message is "bar"
    When they undo the last change
    Then the latest commit message is "foo"
