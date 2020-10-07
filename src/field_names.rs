pub struct FieldNames {
    pub http_version: &'static str,
    pub http_host: &'static str,
    pub referer: &'static str,
    pub remote_address: &'static str,
    pub user_agent: &'static str,
    pub request_method: &'static str,
    pub correlation_id: &'static str,
    pub uri: &'static str,
    pub query_string: &'static str,
    // pub status: &'static str,
    // pub bytes_sent: &'static str,
    // pub response_time: &'static str,
}

impl Default for FieldNames {
    fn default() -> Self {
        FieldNames {
            http_version: "http_version",
            http_host: "http_host",
            referer: "referer",
            remote_address: "remote_address",
            user_agent: "agent",
            request_method: "request_method",
            correlation_id: "correlation-id",
            uri: "uri",
            query_string: "query_string",
            // status: "status",
            // bytes_sent: "bytes_sent",
            // response_time: "response_time",
        }
    }
}
