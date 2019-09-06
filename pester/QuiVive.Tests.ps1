Describe 'create key' {
	BeforeAll {
		$Env:QUI_VIVE_URL | Should -Not -BeNullOrEmpty
		$server_url = $Env:QUI_VIVE_URL
	}
	It 'checks health' {
		$request = Invoke-WebRequest -Uri $server_url/health -Method 'GET' -ContentType 'text/plain'
		$request.StatusCode | Should -Be 200
	}
	It 'creates a key' {
		$url = (Invoke-RestMethod -Uri $server_url/key -Method 'POST' `
			-ContentType 'text/plain' -Body "this is a test").trim()
		$val = Invoke-RestMethod -Uri $url
		$val | Should -Be "this is a test"
	}
	It 'sets a UUID key' {
		$uuid_key = '95ab55a3-d5e2-459d-a11a-e76558eb7a97'
		$url = (Invoke-RestMethod -Uri $server_url/key/$uuid_key -Method 'POST' `
			-ContentType 'text/plain' -Body "PowerShell Rocks!").trim()
		$url | Should -Be $server_url/key/$uuid_key
		$val = Invoke-RestMethod -Uri $server_url/key/$uuid_key
		$val | Should -Be "PowerShell Rocks!"

		$url = (Invoke-RestMethod -Uri $server_url/key/$uuid_key -Method 'POST' `
			-ContentType 'text/plain' -Body "Python is also good").trim()
		$url | Should -Be $server_url/key/$uuid_key
		$val = Invoke-RestMethod -Uri $server_url/key/$uuid_key
		$val | Should -Be "Python is also good"
	}
	It 'create short URL' {
		$long_url = "https://wayk.devolutions.net/"
		$short_url = (Invoke-RestMethod -Uri $server_url/url -Method 'POST' `
			-ContentType 'text/plain' -Body $long_url).trim()
		$request = Invoke-WebRequest -Uri $short_url
		$redirect_url = $request.BaseResponse.RequestMessage.RequestUri.AbsoluteUri
		$redirect_url | Should -Be $long_url
	}
	It 'create invitation link' {
		$dst_url = "https://wayk.devolutions.net/invitation"
		$inv_data = '{"meeting":"master plan","organizer":"ceo@contoso.com"}'
		$content_type = 'application/json'
		$inv_data = "test"
		$headers = @{
			"QuiVive-IdParam" = "id"
			"QuiVive-DstUrl" = $dst_url
		}
		$request = Invoke-WebRequest -Uri $server_url/inv -Method 'POST' `
			-ContentType $content_type -Body $inv_data -Headers $headers
		$request.StatusCode | Should -Be 200
		$short_url = $request.Content
		$short_url | Should -BeLike "$server_url/*"
		$request = Invoke-WebRequest -Uri $short_url
		$redirect_url = $request.BaseResponse.RequestMessage.RequestUri.AbsoluteUri
		$redirect_url | Should -BeLike "$dst_url?id*"
	}
}
