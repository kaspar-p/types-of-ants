use hyper::StatusCode;

pub fn fallback(routes: Vec<&str>) -> (StatusCode, String) {
    (
        StatusCode::NOT_FOUND,
        format!(
            "Unknown route. Valid routes are:\n{}",
            routes
                .iter()
                .map(|&r| String::from(r))
                .map(|r| {
                    return " -> ".to_owned() + &r + &"\n".to_owned();
                })
                .collect::<Vec<String>>()
                .join("")
        ),
    )
}
