#Requires -Version 7.0

$users = 1..50 | ForEach-Object { New-Guid }


# ------------ create poll ----------------

Write-Host "Creating poll"

$createPollBody = @{
    title = "What's the best food?"
    options = @(
        "🍕 Pizza",
        "🍣 Sushi",
        "🍉 Fruit",
        "🍚 Fried rice",
        "🍔 Burgers",
        "🥡 Noodles",
        "🥟 Dumplings",
        "🥗 Salad",
        "🍜 Soup"
    )
    winner_count = 1
}

$poll = $createPollBody | ConvertTo-Json -Depth 3 `
    | Invoke-RestMethod -Method Post -Uri "http://localhost:3000/api/poll" `
        -Headers @{ "User-Id" = $users[0]; } `
        -ContentType "application/json; charset=utf-8"


#------------------ create votes ---------------

$biases = 25, 20, 15, 5, 5, 5, 5, 5, 5 | Get-Random -Shuffle
Write-Host "Biases: $biases"

foreach ($userId in $users) {
    # generate biased poll option list
    $grabBag = $biases `
        | ForEach-Object `
            { $acc = @(); $i = 0; } `
            { $acc += @($i) * $_; $i += 1; } `
            { $acc }

    # take a new unique option from the biased list
    $prefs = @()
    $prefCount = Get-Random -Minimum 1 -Maximum ($poll.option_ids.Length / 2)
    foreach ($i in 0..$prefCount) {
        $selection = $grabBag | Get-Random -Count 1
        $prefs += @($selection)
        $grabBag = $grabBag | Where-Object { $_ -ne $selection }
    }

    $ballot = @{
        ranked_preferences = $prefs
    }

    $ballot | ConvertTo-Json -Depth 3 `
        | Invoke-RestMethod -Method Post -Uri "http://localhost:3000/api/poll/$($poll.id)/my_ballot" `
            -Headers @{ "User-Id" = $userId; } `
            -ContentType "application/json; charset=utf-8" `
        | Out-Null

    $prettyPrefs = $ballot.ranked_preferences `
        | ForEach-Object {
            $poll.options | Where-Object "id" -EQ $_ | Select-Object -ExpandProperty "description"
        } `
        | Join-String -Separator ", "
    Write-Host "User $userId voted for $prettyPrefs"
}


# --------------------- results -----------------------

$result = Invoke-RestMethod -Method Get -Uri "http://localhost:3000/api/poll/$($poll.id)/result"

Write-Host "The results are:"
$result.tally | ForEach-Object {
    $outcome = "-"
    if ($result.winners -contains $_.option_id) {
        $outcome = "✅"
    }
    elseif ($result.eliminated -contains $_.option_id) {
        $outcome = "❌"
    }

    [pscustomobject]@{
        Option = $poll.options | Where-Object id -EQ $_.option_id | Select-Object -ExpandProperty description
        Votes = "$($_.vote_count) / $($result.threshold)"
        Outcome = $outcome
    }
}

# ---------------- cleanup ----------------
Invoke-RestMethod -Method Delete -Uri "http://localhost:3000/api/poll/$($poll.id)" `
    -Headers @{ "User-Id" = $users[0]; }
