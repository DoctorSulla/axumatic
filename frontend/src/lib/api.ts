import { dev } from '$app/environment';

let API_BASE_URL = 'https://api.halliday.dev';

if (dev) {
	API_BASE_URL = 'http://localhost:3000';
}



export interface GoogleLoginRequest {
	jwt: string;
}

export interface LoginRequest {
	email: string;
	password: string;
}

export interface RegisterRequest {
	email: string;
	username: string;
	password: string;
	confirm_password: string;
}

export interface VerifyEmailRequest {
	email: string;
	code: string;
}

export interface ChangePasswordRequest {
	old_password: string;
	password: string;
	confirm_password: string;
}

export interface ConfirmPasswordReset {
	code: string;
	password: string;
	confirm_password: string;
}

export interface ApiResponse {
	response_type: string;
	message: string;
}

async function apiCall(
	endpoint: string,
	method: 'GET' | 'POST' | 'PATCH' | 'POST' | 'DELETE',
	body?: any
): Promise<ApiResponse> {
	try {
		const response = await fetch(`${API_BASE_URL}${endpoint}`, {
			method,
			headers: {
				'Content-Type': 'application/json'
			},
			credentials: 'include',
			body: body ? JSON.stringify(body) : undefined
		});

		if (!response.ok) {
			let error = await response.json().catch(() => {
				return { response_type: 'Error', message: 'Request failed' };
			});
			return error;
		}

		const data = await response.json();
		return data;
	} catch (error) {
		return { response_type: 'Error', message: 'Network request failed' };
	}
}

export const api = {
	async getNonce(): Promise<ApiResponse> {
		return apiCall('/nonce', 'GET', null);
	},

	async login(credentials: LoginRequest): Promise<ApiResponse> {
		return apiCall('/account/login', 'POST', credentials);
	},

	async googleLogin(jwt: GoogleLoginRequest): Promise<ApiResponse> {
		return apiCall('/account/login/google', 'POST', jwt);
	},

	async logout() {
		await fetch(`${API_BASE_URL}/account/logout`, {
			credentials: 'include'
		});
	},

	async register(userData: RegisterRequest): Promise<ApiResponse> {
		return apiCall('/account/register', 'POST', userData);
	},

	async verifyEmail(verifyEmail: VerifyEmailRequest): Promise<ApiResponse> {
		return apiCall('/account/verifyEmail', 'POST', verifyEmail);
	},

	async changePassword(changePasswordRequest: ChangePasswordRequest): Promise<ApiResponse> {
		return apiCall('/account/changePassword', 'PATCH', changePasswordRequest);
	},

	async resetPassword(email: string): Promise<ApiResponse> {
		return apiCall('/account/resetPassword', 'POST', { email });
	},

	async completeResetPassword(confirmPasswordReset: ConfirmPasswordReset): Promise<ApiResponse> {
		return apiCall('/account/resetPassword', 'PATCH', confirmPasswordReset);
	},

	async getProfile(): Promise<ApiResponse> {
		return apiCall('/account/profile', 'GET', null);
	}
};
